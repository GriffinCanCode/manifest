//! High-performance log appenders for different output destinations
//!
//! Provides sophisticated appenders with features like:
//! - Async file writing with configurable buffering
//! - Automatic log rotation with compression
//! - Console output with color support
//! - Network appenders for centralized logging
//! - Performance monitoring and metrics

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use parking_lot::{RwLock, Mutex};
use tokio::sync::mpsc;
use tracing_subscriber::Layer;
use chrono::{DateTime, Utc};
use flate2::{write::GzEncoder, Compression};
use crate::core::hashing::{collections, FastHashMap, HashStrategies};
use super::{ConsoleConfig, FileConfig, RotationConfig, RotationStrategy, LoggingError};

/// Trait for all log appenders
pub trait LogAppender: Send + Sync {
    /// Write a log entry
    fn write(&self, entry: &str) -> Result<(), LoggingError>;
    
    /// Flush pending writes
    fn flush(&self);
    
    /// Rotate logs if applicable
    fn rotate(&self) -> Result<bool, LoggingError>;
    
    /// Create a tracing layer for this appender
    fn create_layer(&self, structured: bool) -> Result<Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync>, LoggingError>;
    
    /// Get appender statistics
    fn stats(&self) -> AppenderStats;
    
    /// Check if appender is healthy
    fn is_healthy(&self) -> bool;
}

/// Statistics for appender monitoring
#[derive(Debug, Clone)]
pub struct AppenderStats {
    pub name: String,
    pub writes_total: u64,
    pub writes_failed: u64,
    pub bytes_written: u64,
    pub last_write: Option<Instant>,
    pub last_rotation: Option<Instant>,
    pub buffer_size: usize,
    pub buffer_used: usize,
}

/// Console appender with color support and async writing
pub struct ConsoleAppender {
    config: ConsoleConfig,
    stats: Arc<RwLock<AppenderStats>>,
    writer: Arc<Mutex<ConsoleWriter>>,
}

impl ConsoleAppender {
    pub fn new(config: ConsoleConfig) -> Result<Self, LoggingError> {
        let stats = Arc::new(RwLock::new(AppenderStats {
            name: "console".to_string(),
            writes_total: 0,
            writes_failed: 0,
            bytes_written: 0,
            last_write: None,
            last_rotation: None,
            buffer_size: 0,
            buffer_used: 0,
        }));
        
        let writer = Arc::new(Mutex::new(ConsoleWriter::new(config.colored)?));
        
        Ok(Self {
            config,
            stats,
            writer,
        })
    }
}

impl LogAppender for ConsoleAppender {
    fn write(&self, entry: &str) -> Result<(), LoggingError> {
        let mut writer = self.writer.lock();
        match writer.write_line(entry) {
            Ok(bytes) => {
                let mut stats = self.stats.write();
                stats.writes_total += 1;
                stats.bytes_written += bytes as u64;
                stats.last_write = Some(Instant::now());
                Ok(())
            }
            Err(e) => {
                let mut stats = self.stats.write();
                stats.writes_failed += 1;
                Err(LoggingError::Io(e))
            }
        }
    }
    
    fn flush(&self) {
        let mut writer = self.writer.lock();
        writer.flush().ok();
    }
    
    fn rotate(&self) -> Result<bool, LoggingError> {
        // Console doesn't rotate
        Ok(false)
    }
    
    fn create_layer(&self, structured: bool) -> Result<Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync>, LoggingError> {
        if structured {
            Ok(Box::new(
                tracing_subscriber::fmt::layer()
                    .with_writer(ConsoleWriterAdapter::new(self.writer.clone()))
                    .with_ansi(self.config.colored)
                    .compact()
            ))
        } else {
            Ok(Box::new(
                tracing_subscriber::fmt::layer()
                    .with_writer(ConsoleWriterAdapter::new(self.writer.clone()))
                    .with_ansi(self.config.colored)
                    .pretty()
            ))
        }
    }
    
    fn stats(&self) -> AppenderStats {
        self.stats.read().clone()
    }
    
    fn is_healthy(&self) -> bool {
        true // Console is always healthy
    }
}

/// File appender with async writing, buffering, and rotation
pub struct FileAppender {
    config: FileConfig,
    stats: Arc<RwLock<AppenderStats>>,
    writer: Arc<AsyncFileWriter>,
}

impl FileAppender {
    pub fn new(config: FileConfig) -> Result<Self, LoggingError> {
        let current_path = Self::resolve_path_template(&config.path)?;
        
        // Ensure directory exists
        if let Some(parent) = current_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let stats = Arc::new(RwLock::new(AppenderStats {
            name: format!("file:{}", current_path.display()),
            writes_total: 0,
            writes_failed: 0,
            bytes_written: 0,
            last_write: None,
            last_rotation: None,
            buffer_size: config.buffer_size,
            buffer_used: 0,
        }));
        
        let writer = Arc::new(AsyncFileWriter::new(
            current_path,
            config.buffer_size,
            Duration::from_millis(config.flush_interval_ms),
            stats.clone(),
        )?);
        
        Ok(Self {
            config,
            stats,
            writer,
        })
    }
    
    fn resolve_path_template(template: &Path) -> Result<PathBuf, LoggingError> {
        let template_str = template.to_string_lossy();
        let now = Utc::now();
        
        let resolved = template_str
            .replace("{date}", &now.format("%Y-%m-%d").to_string())
            .replace("{time}", &now.format("%H-%M-%S").to_string())
            .replace("{pid}", &std::process::id().to_string())
            .replace("{timestamp}", &now.timestamp().to_string());
        
        Ok(PathBuf::from(resolved))
    }
    
    fn should_rotate(&self) -> Result<bool, LoggingError> {
        match &self.config.rotation.strategy {
            RotationStrategy::Never => Ok(false),
            RotationStrategy::Daily => {
                let stats = self.stats.read();
                if let Some(last_rotation) = stats.last_rotation {
                    let elapsed = Instant::now().duration_since(last_rotation);
                    Ok(elapsed >= Duration::from_secs(24 * 60 * 60))
                } else {
                    // Check file creation time
                    let path = Self::resolve_path_template(&self.config.path)?;
                    if path.exists() {
                        let metadata = std::fs::metadata(&path)?;
                        let created = metadata.created().or_else(|_| metadata.modified())?;
                        let elapsed = SystemTime::now().duration_since(created)
                            .map_err(|e| LoggingError::Io(io::Error::new(io::ErrorKind::Other, e)))?;
                        Ok(elapsed >= Duration::from_secs(24 * 60 * 60))
                    } else {
                        Ok(false)
                    }
                }
            }
            RotationStrategy::Hourly => {
                let stats = self.stats.read();
                if let Some(last_rotation) = stats.last_rotation {
                    let elapsed = Instant::now().duration_since(last_rotation);
                    Ok(elapsed >= Duration::from_secs(60 * 60))
                } else {
                    Ok(true) // First rotation
                }
            }
            RotationStrategy::Size(max_size) => {
                let path = Self::resolve_path_template(&self.config.path)?;
                if path.exists() {
                    let metadata = std::fs::metadata(&path)?;
                    Ok(metadata.len() >= *max_size)
                } else {
                    Ok(false)
                }
            }
            RotationStrategy::Time(interval_secs) => {
                let stats = self.stats.read();
                if let Some(last_rotation) = stats.last_rotation {
                    let elapsed = Instant::now().duration_since(last_rotation);
                    Ok(elapsed >= Duration::from_secs(*interval_secs))
                } else {
                    Ok(true) // First rotation
                }
            }
        }
    }
    
    fn perform_rotation(&self) -> Result<(), LoggingError> {
        let current_path = Self::resolve_path_template(&self.config.path)?;
        
        if !current_path.exists() {
            return Ok(());
        }
        
        // Generate archive name
        let now = Utc::now();
        let archive_name = format!(
            "{}.{}{}",
            current_path.display(),
            now.format("%Y%m%d-%H%M%S"),
            if self.config.rotation.compress { ".gz" } else { "" }
        );
        let archive_path = current_path.with_file_name(archive_name);
        
        // Stop writing temporarily
        self.writer.pause_writing();
        
        if self.config.rotation.compress {
            // Compress the file
            let input = File::open(&current_path)?;
            let output = File::create(&archive_path)?;
            let mut encoder = GzEncoder::new(output, Compression::default());
            
            let mut reader = BufReader::new(input);
            let mut buffer = String::new();
            
            while reader.read_line(&mut buffer)? > 0 {
                encoder.write_all(buffer.as_bytes())?;
                buffer.clear();
            }
            
            encoder.finish()?;
        } else {
            // Just rename the file
            std::fs::rename(&current_path, &archive_path)?;
        }
        
        // Remove the original file
        if current_path.exists() {
            std::fs::remove_file(&current_path)?;
        }
        
        // Resume writing with new file
        self.writer.resume_writing(&current_path)?;
        
        // Clean up old archives
        self.cleanup_old_archives()?;
        
        // Update stats
        {
            let mut stats = self.stats.write();
            stats.last_rotation = Some(Instant::now());
        }
        
        Ok(())
    }
    
    fn cleanup_old_archives(&self) -> Result<(), LoggingError> {
        let current_path = Self::resolve_path_template(&self.config.path)?;
        let dir = current_path.parent().ok_or_else(|| {
            LoggingError::Io(io::Error::new(io::ErrorKind::InvalidInput, "Invalid log path"))
        })?;
        
        if !dir.exists() {
            return Ok(());
        }
        
        // Find all archive files
        let prefix = current_path.file_name().unwrap().to_string_lossy();
        let mut archives: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.file_name().to_string_lossy().starts_with(&*prefix) &&
                entry.path() != current_path
            })
            .collect();
        
        // Sort by modification time
        archives.sort_by_key(|entry| {
            entry.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        });
        
        // Remove excess archives
        if archives.len() > self.config.rotation.max_archives {
            let to_remove = archives.len() - self.config.rotation.max_archives;
            for archive in archives.iter().take(to_remove) {
                std::fs::remove_file(archive.path())?;
            }
        }
        
        // Remove archives older than cleanup_days
        if let Some(cleanup_days) = self.config.rotation.cleanup_days {
            let cutoff = SystemTime::now() - Duration::from_secs(cleanup_days as u64 * 24 * 60 * 60);
            
            for archive in &archives {
                if let Ok(metadata) = archive.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if modified < cutoff {
                            std::fs::remove_file(archive.path())?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

impl LogAppender for FileAppender {
    fn write(&self, entry: &str) -> Result<(), LoggingError> {
        self.writer.write(entry)
    }
    
    fn flush(&self) {
        self.writer.flush();
    }
    
    fn rotate(&self) -> Result<bool, LoggingError> {
        if self.should_rotate()? {
            self.perform_rotation()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    fn create_layer(&self, structured: bool) -> Result<Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync>, LoggingError> {
        let writer_adapter = FileWriterAdapter::new(self.writer.clone());
        
        if structured {
            Ok(Box::new(
                tracing_subscriber::fmt::layer()
                    .with_writer(writer_adapter)
                    .with_ansi(false)
                    .json()
            ))
        } else {
            Ok(Box::new(
                tracing_subscriber::fmt::layer()
                    .with_writer(writer_adapter)
                    .with_ansi(false)
            ))
        }
    }
    
    fn stats(&self) -> AppenderStats {
        self.stats.read().clone()
    }
    
    fn is_healthy(&self) -> bool {
        self.writer.is_healthy()
    }
}

/// Async file writer with buffering
pub struct AsyncFileWriter {
    sender: mpsc::UnboundedSender<WriteCommand>,
    stats: Arc<RwLock<AppenderStats>>,
    healthy: Arc<parking_lot::RwLock<bool>>,
}

enum WriteCommand {
    Write(String),
    Flush,
    Pause,
    Resume(PathBuf),
    Shutdown,
}

impl AsyncFileWriter {
    pub fn new(
        path: PathBuf,
        buffer_size: usize,
        flush_interval: Duration,
        stats: Arc<RwLock<AppenderStats>>,
    ) -> Result<Self, LoggingError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let healthy = Arc::new(parking_lot::RwLock::new(true));
        
        // Spawn writer task
        let writer_stats = stats.clone();
        let writer_healthy = healthy.clone();
        tokio::spawn(async move {
            let mut writer = FileWriterTask::new(path, buffer_size, flush_interval, writer_stats, writer_healthy);
            writer.run(receiver).await;
        });
        
        Ok(Self {
            sender,
            stats,
            healthy,
        })
    }
    
    pub fn write(&self, entry: &str) -> Result<(), LoggingError> {
        self.sender
            .send(WriteCommand::Write(entry.to_string()))
            .map_err(|_| LoggingError::Io(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Writer task shutdown"
            )))
    }
    
    pub fn flush(&self) {
        self.sender.send(WriteCommand::Flush).ok();
    }
    
    pub fn pause_writing(&self) {
        self.sender.send(WriteCommand::Pause).ok();
    }
    
    pub fn resume_writing(&self, new_path: &Path) -> Result<(), LoggingError> {
        self.sender
            .send(WriteCommand::Resume(new_path.to_path_buf()))
            .map_err(|_| LoggingError::Io(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Writer task shutdown"
            )))
    }
    
    pub fn is_healthy(&self) -> bool {
        *self.healthy.read()
    }
}

impl Drop for AsyncFileWriter {
    fn drop(&mut self) {
        self.sender.send(WriteCommand::Shutdown).ok();
    }
}

/// Internal file writer task
struct FileWriterTask {
    current_path: PathBuf,
    writer: Option<BufWriter<File>>,
    buffer_size: usize,
    flush_interval: Duration,
    last_flush: Instant,
    stats: Arc<RwLock<AppenderStats>>,
    healthy: Arc<parking_lot::RwLock<bool>>,
    paused: bool,
}

impl FileWriterTask {
    fn new(
        path: PathBuf,
        buffer_size: usize,
        flush_interval: Duration,
        stats: Arc<RwLock<AppenderStats>>,
        healthy: Arc<parking_lot::RwLock<bool>>,
    ) -> Self {
        Self {
            current_path: path,
            writer: None,
            buffer_size,
            flush_interval,
            last_flush: Instant::now(),
            stats,
            healthy,
            paused: false,
        }
    }
    
    async fn run(&mut self, mut receiver: mpsc::UnboundedReceiver<WriteCommand>) {
        let mut flush_timer = tokio::time::interval(self.flush_interval);
        
        loop {
            tokio::select! {
                command = receiver.recv() => {
                    match command {
                        Some(WriteCommand::Write(entry)) => {
                            if !self.paused {
                                self.handle_write(&entry).await;
                            }
                        }
                        Some(WriteCommand::Flush) => {
                            self.handle_flush().await;
                        }
                        Some(WriteCommand::Pause) => {
                            self.paused = true;
                            self.handle_flush().await;
                            if let Some(writer) = self.writer.take() {
                                drop(writer);
                            }
                        }
                        Some(WriteCommand::Resume(new_path)) => {
                            self.current_path = new_path;
                            self.paused = false;
                            self.ensure_writer().await;
                        }
                        Some(WriteCommand::Shutdown) | None => {
                            self.handle_flush().await;
                            break;
                        }
                    }
                }
                _ = flush_timer.tick() => {
                    if Instant::now().duration_since(self.last_flush) >= self.flush_interval {
                        self.handle_flush().await;
                    }
                }
            }
        }
    }
    
    async fn handle_write(&mut self, entry: &str) {
        if self.ensure_writer().await {
            if let Some(writer) = &mut self.writer {
                match writer.write_all(entry.as_bytes()) {
                    Ok(()) => {
                        let mut stats = self.stats.write();
                        stats.writes_total += 1;
                        stats.bytes_written += entry.len() as u64;
                        stats.last_write = Some(Instant::now());
                        stats.buffer_used = writer.buffer().len();
                    }
                    Err(_) => {
                        let mut stats = self.stats.write();
                        stats.writes_failed += 1;
                        *self.healthy.write() = false;
                    }
                }
            }
        }
    }
    
    async fn handle_flush(&mut self) {
        if let Some(writer) = &mut self.writer {
            if writer.flush().is_ok() {
                self.last_flush = Instant::now();
                let mut stats = self.stats.write();
                stats.buffer_used = 0;
            }
        }
    }
    
    async fn ensure_writer(&mut self) -> bool {
        if self.writer.is_none() {
            match self.create_writer() {
                Ok(writer) => {
                    self.writer = Some(writer);
                    *self.healthy.write() = true;
                }
                Err(_) => {
                    *self.healthy.write() = false;
                    return false;
                }
            }
        }
        true
    }
    
    fn create_writer(&self) -> Result<BufWriter<File>, LoggingError> {
        if let Some(parent) = self.current_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.current_path)?;
        
        Ok(BufWriter::with_capacity(self.buffer_size, file))
    }
}

/// Console writer with color support
struct ConsoleWriter {
    colored: bool,
}

impl ConsoleWriter {
    fn new(colored: bool) -> Result<Self, LoggingError> {
        Ok(Self { colored })
    }
    
    fn write_line(&mut self, line: &str) -> Result<usize, io::Error> {
        use std::io::{stdout, Write};
        
        let mut stdout = stdout();
        let bytes = line.len();
        
        stdout.write_all(line.as_bytes())?;
        if !line.ends_with('\n') {
            stdout.write_all(b"\n")?;
        }
        stdout.flush()?;
        
        Ok(bytes)
    }
    
    fn flush(&mut self) -> Result<(), io::Error> {
        use std::io::{stdout, Write};
        stdout().flush()
    }
}

/// Writer adapter for console
#[derive(Clone)]
struct ConsoleWriterAdapter {
    writer: Arc<Mutex<ConsoleWriter>>,
}

impl ConsoleWriterAdapter {
    fn new(writer: Arc<Mutex<ConsoleWriter>>) -> Self {
        Self { writer }
    }
}

impl Write for ConsoleWriterAdapter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        let mut writer = self.writer.lock();
        writer.write_line(&s)
    }
    
    fn flush(&mut self) -> io::Result<()> {
        let mut writer = self.writer.lock();
        writer.flush()
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for ConsoleWriterAdapter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Writer adapter for file
#[derive(Clone)]
struct FileWriterAdapter {
    writer: Arc<AsyncFileWriter>,
}

impl FileWriterAdapter {
    fn new(writer: Arc<AsyncFileWriter>) -> Self {
        Self { writer }
    }
}

impl Write for FileWriterAdapter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        match self.writer.write(&s) {
            Ok(()) => Ok(buf.len()),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }
    
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush();
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for FileWriterAdapter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_console_appender() {
        let config = ConsoleConfig::default();
        let appender = ConsoleAppender::new(config).unwrap();
        
        assert!(appender.write("test message\n").is_ok());
        assert!(appender.is_healthy());
        
        let stats = appender.stats();
        assert_eq!(stats.writes_total, 1);
        assert!(stats.bytes_written > 0);
    }
    
    #[test]
    fn test_path_template_resolution() {
        let template = PathBuf::from("logs/test-{date}-{pid}.log");
        let resolved = FileAppender::resolve_path_template(&template).unwrap();
        
        let resolved_str = resolved.to_string_lossy();
        assert!(resolved_str.contains("logs/test-"));
        assert!(resolved_str.contains(&std::process::id().to_string()));
        assert!(resolved_str.ends_with(".log"));
    }
    
    #[tokio::test]
    async fn test_file_appender() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        
        let mut config = FileConfig::new_game_log();
        config.path = log_path.clone();
        config.buffer_size = 1024;
        config.flush_interval_ms = 100;
        
        let appender = FileAppender::new(config).unwrap();
        
        assert!(appender.write("test message 1\n").is_ok());
        assert!(appender.write("test message 2\n").is_ok());
        
        // Give async writer time to write
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        appender.flush();
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        assert!(log_path.exists());
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("test message 1"));
        assert!(content.contains("test message 2"));
    }
    
    #[test]
    fn test_rotation_strategies() {
        // Test size-based rotation
        let size_rotation = RotationConfig::size_based(1024);
        assert!(matches!(size_rotation.strategy, RotationStrategy::Size(1024)));
        
        // Test daily rotation
        let daily_rotation = RotationConfig::daily();
        assert!(matches!(daily_rotation.strategy, RotationStrategy::Daily));
        assert_eq!(daily_rotation.max_archives, 30);
        assert!(daily_rotation.compress);
    }
}
