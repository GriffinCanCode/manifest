use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use chrono::{DateTime, Utc};

/// Simple debug logger for development that writes to files
/// This complements the main logging system for debugging freezes and crashes
pub struct DebugLogger {
    log_file: Option<File>,
}

impl DebugLogger {
    pub fn new() -> Self {
        let log_file = Self::create_log_file();
        let mut logger = DebugLogger { log_file };
        
        if logger.log_file.is_some() {
            logger.write_log("DEBUG", "Logger initialized", None);
        }
        
        logger
    }
    
    fn create_log_file() -> Option<File> {
        let logs_dir = Path::new("logs");
        if !logs_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(logs_dir) {
                eprintln!("Failed to create logs directory: {}", e);
                return None;
            }
        }
        
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let log_path = logs_dir.join(format!("debug-{}.log", timestamp));
        
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
        {
            Ok(file) => Some(file),
            Err(e) => {
                eprintln!("Failed to create log file: {}", e);
                None
            }
        }
    }
    
    fn write_log(&mut self, level: &str, message: &str, context: Option<&str>) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = match context {
            Some(ctx) => format!("[{}] {} [{}] {}\n", timestamp, level, ctx, message),
            None => format!("[{}] {} {}\n", timestamp, level, message),
        };
        
        // Always print to stderr for console visibility
        eprint!("{}", log_line);
        
        // Also write to file if available
        if let Some(ref mut file) = self.log_file {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
    }
    
    pub fn info(&mut self, message: &str, context: Option<&str>) {
        self.write_log("INFO", message, context);
    }
    
    pub fn warn(&mut self, message: &str, context: Option<&str>) {
        self.write_log("WARN", message, context);
    }
    
    pub fn error(&mut self, message: &str, context: Option<&str>) {
        self.write_log("ERROR", message, context);
    }
    
    pub fn debug(&mut self, message: &str, context: Option<&str>) {
        self.write_log("DEBUG", message, context);
    }
    
    pub fn performance(&mut self, operation: &str, duration_ms: f64, context: Option<&str>) {
        let message = format!("PERF: {} took {:.2}ms", operation, duration_ms);
        self.write_log("PERF", &message, context);
    }
    
    pub fn freeze_detector(&mut self, component: &str) {
        let message = format!("Potential freeze detected in {}", component);
        self.write_log("FREEZE", &message, Some("CRITICAL"));
    }
}

impl Default for DebugLogger {
    fn default() -> Self {
        Self::new()
    }
}

// Thread-local logger for easy access
thread_local! {
    static DEBUG_LOGGER: std::cell::RefCell<DebugLogger> = std::cell::RefCell::new(DebugLogger::new());
}

// Convenience macros
#[macro_export]
macro_rules! debug_log {
    ($level:ident, $msg:expr) => {
        $crate::core::logging::debug_logger::DEBUG_LOGGER.with(|logger| {
            logger.borrow_mut().$level($msg, None);
        });
    };
    ($level:ident, $msg:expr, $ctx:expr) => {
        $crate::core::logging::debug_logger::DEBUG_LOGGER.with(|logger| {
            logger.borrow_mut().$level($msg, Some($ctx));
        });
    };
}

// Specific macros for common use cases
#[macro_export]
macro_rules! freeze_check {
    ($component:expr) => {
        $crate::core::logging::debug_logger::DEBUG_LOGGER.with(|logger| {
            logger.borrow_mut().freeze_detector($component);
        });
    };
}

#[macro_export]
macro_rules! perf_log {
    ($operation:expr, $duration:expr) => {
        $crate::core::logging::debug_logger::DEBUG_LOGGER.with(|logger| {
            logger.borrow_mut().performance($operation, $duration, None);
        });
    };
    ($operation:expr, $duration:expr, $ctx:expr) => {
        $crate::core::logging::debug_logger::DEBUG_LOGGER.with(|logger| {
            logger.borrow_mut().performance($operation, $duration, Some($ctx));
        });
    };
}
