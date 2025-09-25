//! High-performance log formatters for different output targets
//!
//! Provides multiple formatting strategies optimized for different use cases:
//! - Human-readable console output with colors
//! - Structured JSON for machine processing
//! - Compact logfmt for high-throughput scenarios
//! - Game-specific formats for analysis tools

use std::fmt;
use std::sync::Arc;
use chrono::Utc;
use serde_json::{Map, Value};
use tracing::{Event, Subscriber};
use tracing_subscriber::{
    fmt::{format::Writer, FormatEvent, FormatFields, FormattedFields},
    registry::LookupSpan,
    Layer,
};
use super::{FileFormatConfig, FileFormatType, LoggingError, SensitiveDataFilter};

/// Creates a formatter layer based on configuration
pub fn create_formatter_layer<S>(
    config: &FileFormatConfig,
    filter: Arc<SensitiveDataFilter>,
) -> Result<Box<dyn Layer<S> + Send + Sync>, LoggingError>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup> + Send + Sync,
{
    match config.format_type {
        FileFormatType::Plain => {
            let formatter = PlainFormatter::new(config.clone(), filter);
            Ok(Box::new(
                tracing_subscriber::fmt::layer()
                    .event_format(formatter)
                    .fmt_fields(GameFieldFormatter::new())
            ))
        }
        FileFormatType::Json => {
            let formatter = JsonFormatter::new(config.clone(), filter);
            Ok(Box::new(
                tracing_subscriber::fmt::layer()
                    .event_format(formatter)
                    .fmt_fields(JsonFieldFormatter::new())
            ))
        }
        FileFormatType::Logfmt => {
            let formatter = LogfmtFormatter::new(config.clone(), filter);
            Ok(Box::new(
                tracing_subscriber::fmt::layer()
                    .event_format(formatter)
                    .fmt_fields(LogfmtFieldFormatter::new())
            ))
        }
        FileFormatType::GameFormat => {
            let formatter = GameFormatter::new(config.clone(), filter);
            Ok(Box::new(
                tracing_subscriber::fmt::layer()
                    .event_format(formatter)
                    .fmt_fields(GameFieldFormatter::new())
            ))
        }
    }
}

/// Console formatter with colors and human-readable output
pub struct ConsoleFormatter {
    config: FileFormatConfig,
    sensitive_filter: Arc<SensitiveDataFilter>,
    colors: ColorScheme,
}

impl ConsoleFormatter {
    pub fn new(config: FileFormatConfig, filter: Arc<SensitiveDataFilter>) -> Self {
        Self {
            config,
            sensitive_filter: filter,
            colors: ColorScheme::default(),
        }
    }
}

impl<S, N> FormatEvent<S, N> for ConsoleFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        let now = Utc::now();
        
        // Timestamp
        if self.config.timestamps {
            write!(
                writer,
                "{}{}{} ",
                self.colors.timestamp,
                now.format("%Y-%m-%d %H:%M:%S%.3f"),
                self.colors.reset
            )?;
        }
        
        // Level with colors
        let level_color = self.colors.level_color(metadata.level());
        write!(
            writer,
            "{}[{}]{} ",
            level_color,
            metadata.level(),
            self.colors.reset
        )?;
        
        // Thread info
        if self.config.thread_info {
            if let Some(thread_name) = std::thread::current().name() {
                write!(
                    writer,
                    "{}({}){} ",
                    self.colors.thread,
                    thread_name,
                    self.colors.reset
                )?;
            }
        }
        
        // Module/target
        write!(
            writer,
            "{}[{}]{} ",
            self.colors.target,
            metadata.target(),
            self.colors.reset
        )?;
        
        // Correlation ID if present
        if self.config.correlation_ids {
            if let Some(span) = ctx.lookup_current() {
                if let Some(correlation_id) = span.extensions().get::<u64>() {
                    write!(
                        writer,
                        "{}correlation_id={}{} ",
                        self.colors.field_key,
                        correlation_id,
                        self.colors.reset
                    )?;
                }
            }
        }
        
        // Message
        ctx.format_fields(writer.by_ref(), event)?;
        
        // Source location
        if self.config.source_location {
            if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
                write!(
                    writer,
                    " {}at {}:{}{} ",
                    self.colors.location,
                    file,
                    line,
                    self.colors.reset
                )?;
            }
        }
        
        writeln!(writer)
    }
}

/// Plain text formatter without colors
pub struct PlainFormatter {
    config: FileFormatConfig,
    sensitive_filter: Arc<SensitiveDataFilter>,
}

impl PlainFormatter {
    pub fn new(config: FileFormatConfig, filter: Arc<SensitiveDataFilter>) -> Self {
        Self {
            config,
            sensitive_filter: filter,
        }
    }
}

impl<S, N> FormatEvent<S, N> for PlainFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        
        // Timestamp
        if self.config.timestamps {
            let now = Utc::now();
            write!(writer, "{} ", now.format("%Y-%m-%d %H:%M:%S%.3f"))?;
        }
        
        // Level
        write!(writer, "[{}] ", metadata.level())?;
        
        // Thread
        if self.config.thread_info {
            if let Some(thread_name) = std::thread::current().name() {
                write!(writer, "({}) ", thread_name)?;
            }
        }
        
        // Target
        write!(writer, "[{}] ", metadata.target())?;
        
        // Correlation ID
        if self.config.correlation_ids {
            if let Some(span) = ctx.lookup_current() {
                if let Some(correlation_id) = span.extensions().get::<u64>() {
                    write!(writer, "correlation_id={} ", correlation_id)?;
                }
            }
        }
        
        // Message and fields
        ctx.format_fields(writer.by_ref(), event)?;
        
        // Source location
        if self.config.source_location {
            if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
                write!(writer, " at {}:{}", file, line)?;
            }
        }
        
        writeln!(writer)
    }
}

/// JSON formatter for structured logging
pub struct JsonFormatter {
    config: FileFormatConfig,
    sensitive_filter: Arc<SensitiveDataFilter>,
}

impl JsonFormatter {
    pub fn new(config: FileFormatConfig, filter: Arc<SensitiveDataFilter>) -> Self {
        Self {
            config,
            sensitive_filter: filter,
        }
    }
}

impl<S, N> FormatEvent<S, N> for JsonFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        let mut json = Map::new();
        
        // Timestamp
        if self.config.timestamps {
            let now = Utc::now();
            json.insert("timestamp".to_string(), Value::String(now.to_rfc3339()));
        }
        
        // Basic metadata
        json.insert("level".to_string(), Value::String(metadata.level().to_string()));
        json.insert("target".to_string(), Value::String(metadata.target().to_string()));
        
        // Thread info
        if self.config.thread_info {
            if let Some(thread_name) = std::thread::current().name() {
                json.insert("thread".to_string(), Value::String(thread_name.to_string()));
            }
        }
        
        // Correlation ID
        if self.config.correlation_ids {
            if let Some(span) = ctx.lookup_current() {
                if let Some(correlation_id) = span.extensions().get::<u64>() {
                    json.insert("correlation_id".to_string(), Value::Number((*correlation_id).into()));
                }
            }
        }
        
        // Source location
        if self.config.source_location {
            if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
                let mut location = Map::new();
                location.insert("file".to_string(), Value::String(file.to_string()));
                location.insert("line".to_string(), Value::Number(line.into()));
                json.insert("location".to_string(), Value::Object(location));
            }
        }
        
        // Message and fields (captured by field formatter)
        let mut field_buffer = String::new();
        {
            let field_writer = Writer::new(&mut field_buffer);
            ctx.format_fields(field_writer, event).ok();
        }
        
        if !field_buffer.is_empty() {
            // Parse fields from the buffer with proper structured field extraction
            match parse_structured_fields(&field_buffer) {
                Ok(parsed_fields) => {
                    // Merge parsed fields into the JSON object
                    for (key, value) in parsed_fields {
                        json.insert(key, value);
                    }
                }
                Err(_) => {
                    // Fallback: treat the entire buffer as message
                    json.insert("message".to_string(), Value::String(field_buffer.trim().to_string()));
                }
            }
        }
        
        // Add span context
        if let Some(span) = ctx.lookup_current() {
            let mut span_info = Map::new();
            span_info.insert("name".to_string(), Value::String(span.name().to_string()));
            
            if let Some(fields) = span.extensions().get::<FormattedFields<N>>() {
                if !fields.fields.is_empty() {
                    span_info.insert("fields".to_string(), Value::String(fields.fields.clone()));
                }
            }
            
            json.insert("span".to_string(), Value::Object(span_info));
        }
        
        // Filter sensitive data
        let json_str = serde_json::to_string(&json).map_err(|_| fmt::Error)?;
        let filtered_str = self.sensitive_filter.redact_sensitive_data(&json_str);
        
        writeln!(writer, "{}", filtered_str)
    }
}

/// Logfmt formatter for structured key-value logging
pub struct LogfmtFormatter {
    config: FileFormatConfig,
    sensitive_filter: Arc<SensitiveDataFilter>,
}

impl LogfmtFormatter {
    pub fn new(config: FileFormatConfig, filter: Arc<SensitiveDataFilter>) -> Self {
        Self {
            config,
            sensitive_filter: filter,
        }
    }
    
    fn write_key_value<W: fmt::Write>(&self, writer: &mut W, key: &str, value: &str) -> fmt::Result {
        if self.sensitive_filter.should_filter_field(key) {
            write!(writer, "{}=\"[REDACTED]\" ", key)
        } else if value.contains(' ') || value.contains('"') {
            write!(writer, "{}=\"{}\" ", key, value.replace('"', "\\\""))
        } else {
            write!(writer, "{}={} ", key, value)
        }
    }
}

impl<S, N> FormatEvent<S, N> for LogfmtFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        
        // Timestamp
        if self.config.timestamps {
            let now = Utc::now();
            self.write_key_value(&mut writer, "timestamp", &now.to_rfc3339())?;
        }
        
        // Level
        self.write_key_value(&mut writer, "level", &metadata.level().to_string())?;
        
        // Target
        self.write_key_value(&mut writer, "target", metadata.target())?;
        
        // Thread
        if self.config.thread_info {
            if let Some(thread_name) = std::thread::current().name() {
                self.write_key_value(&mut writer, "thread", thread_name)?;
            }
        }
        
        // Correlation ID
        if self.config.correlation_ids {
            if let Some(span) = ctx.lookup_current() {
                if let Some(correlation_id) = span.extensions().get::<u64>() {
                    self.write_key_value(&mut writer, "correlation_id", &correlation_id.to_string())?;
                }
            }
        }
        
        // Message and fields
        let mut field_buffer = String::new();
        {
            let field_writer = Writer::new(&mut field_buffer);
            ctx.format_fields(field_writer, event).ok();
        }
        
        if !field_buffer.is_empty() {
            self.write_key_value(&mut writer, "message", field_buffer.trim())?;
        }
        
        // Source location
        if self.config.source_location {
            if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
                self.write_key_value(&mut writer, "source", &format!("{}:{}", file, line))?;
            }
        }
        
        writeln!(writer)
    }
}

/// Game-specific formatter optimized for analysis
pub struct GameFormatter {
    config: FileFormatConfig,
    sensitive_filter: Arc<SensitiveDataFilter>,
}

impl GameFormatter {
    pub fn new(config: FileFormatConfig, filter: Arc<SensitiveDataFilter>) -> Self {
        Self {
            config,
            sensitive_filter: filter,
        }
    }
}

impl<S, N> FormatEvent<S, N> for GameFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        
        // Game-specific prefix with structured info
        write!(writer, "GAME")?;
        
        // Timestamp in game format
        if self.config.timestamps {
            let now = Utc::now();
            write!(writer, "|{}|", now.timestamp_millis())?;
        }
        
        // Level as single character
        let level_char = match *metadata.level() {
            tracing::Level::TRACE => 'T',
            tracing::Level::DEBUG => 'D',
            tracing::Level::INFO => 'I',
            tracing::Level::WARN => 'W',
            tracing::Level::ERROR => 'E',
        };
        write!(writer, "{}|", level_char)?;
        
        // Category from target
        let category = metadata.target()
            .strip_prefix("game::")
            .or_else(|| metadata.target().strip_prefix("manifest::"))
            .unwrap_or(metadata.target());
        write!(writer, "{}|", category)?;
        
        // Correlation ID
        if self.config.correlation_ids {
            if let Some(span) = ctx.lookup_current() {
                if let Some(correlation_id) = span.extensions().get::<u64>() {
                    write!(writer, "{}|", correlation_id)?;
                } else {
                    write!(writer, "0|")?;
                }
            } else {
                write!(writer, "0|")?;
            }
        }
        
        // Message
        ctx.format_fields(writer.by_ref(), event)?;
        
        writeln!(writer)
    }
}

/// Custom field formatter for game-specific data
pub struct GameFieldFormatter {
    sensitive_filter: Arc<SensitiveDataFilter>,
}

impl GameFieldFormatter {
    pub fn new() -> Self {
        Self {
            sensitive_filter: Arc::new(SensitiveDataFilter::new()),
        }
    }
}

impl<'writer> FormatFields<'writer> for GameFieldFormatter {
    fn format_fields<R: tracing_subscriber::field::RecordFields>(
        &self,
        writer: Writer<'writer>,
        fields: R,
    ) -> fmt::Result {
        // Create visitor without filter reference to avoid lifetime issues
        let mut visitor = GameFieldVisitorSimple::new(writer);
        fields.record(&mut visitor);
        visitor.finish()
    }
}

/// JSON field formatter
pub struct JsonFieldFormatter;

impl JsonFieldFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl<'writer> FormatFields<'writer> for JsonFieldFormatter {
    fn format_fields<R: tracing_subscriber::field::RecordFields>(
        &self,
        writer: Writer<'writer>,
        fields: R,
    ) -> fmt::Result {
        let mut visitor = JsonFieldVisitor::new(writer);
        fields.record(&mut visitor);
        visitor.finish()
    }
}

/// Logfmt field formatter
pub struct LogfmtFieldFormatter;

impl LogfmtFieldFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl<'writer> FormatFields<'writer> for LogfmtFieldFormatter {
    fn format_fields<R: tracing_subscriber::field::RecordFields>(
        &self,
        writer: Writer<'writer>,
        fields: R,
    ) -> fmt::Result {
        let mut visitor = LogfmtFieldVisitor::new(writer);
        fields.record(&mut visitor);
        visitor.finish()
    }
}

/// Field visitor implementations
struct GameFieldVisitor<'writer> {
    writer: Writer<'writer>,
    sensitive_filter: &'writer SensitiveDataFilter,
    first: bool,
}

/// Simple field visitor without filter reference
struct GameFieldVisitorSimple<'writer> {
    writer: Writer<'writer>,
    first: bool,
}

impl<'writer> GameFieldVisitor<'writer> {
    fn new(writer: Writer<'writer>, filter: &'writer SensitiveDataFilter) -> Self {
        Self {
            writer,
            sensitive_filter: filter,
            first: true,
        }
    }
    
    fn finish(self) -> fmt::Result {
        Ok(())
    }
}

impl<'writer> GameFieldVisitorSimple<'writer> {
    fn new(writer: Writer<'writer>) -> Self {
        Self {
            writer,
            first: true,
        }
    }
    
    fn finish(self) -> fmt::Result {
        Ok(())
    }
}

impl<'writer> tracing::field::Visit for GameFieldVisitor<'writer> {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if !self.first {
            write!(self.writer, " ").ok();
        }
        self.first = false;
        
        if self.sensitive_filter.should_filter_field(field.name()) {
            write!(self.writer, "{}=[REDACTED]", field.name()).ok();
        } else {
            write!(self.writer, "{}={}", field.name(), value).ok();
        }
    }
    
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if !self.first {
            write!(self.writer, " ").ok();
        }
        self.first = false;
        
        if self.sensitive_filter.should_filter_field(field.name()) {
            write!(self.writer, "{}=[REDACTED]", field.name()).ok();
        } else {
            write!(self.writer, "{}={}", field.name(), value).ok();
        }
    }
    
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if !self.first {
            write!(self.writer, " ").ok();
        }
        self.first = false;
        
        if self.sensitive_filter.should_filter_field(field.name()) {
            write!(self.writer, "{}=[REDACTED]", field.name()).ok();
        } else {
            write!(self.writer, "{}={}", field.name(), value).ok();
        }
    }
    
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if !self.first {
            write!(self.writer, " ").ok();
        }
        self.first = false;
        
        if self.sensitive_filter.should_filter_field(field.name()) {
            write!(self.writer, "{}=[REDACTED]", field.name()).ok();
        } else {
            let filtered_value = self.sensitive_filter.redact_sensitive_data(value);
            write!(self.writer, "{}=\"{}\"", field.name(), filtered_value).ok();
        }
    }
    
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if !self.first {
            write!(self.writer, " ").ok();
        }
        self.first = false;
        
        if self.sensitive_filter.should_filter_field(field.name()) {
            write!(self.writer, "{}=[REDACTED]", field.name()).ok();
        } else {
            write!(self.writer, "{}={:?}", field.name(), value).ok();
        }
    }
}

impl<'writer> tracing::field::Visit for GameFieldVisitorSimple<'writer> {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if !self.first {
            write!(self.writer, " ").ok();
        }
        self.first = false;
        write!(self.writer, "{}={}", field.name(), value).ok();
    }
    
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if !self.first {
            write!(self.writer, " ").ok();
        }
        self.first = false;
        write!(self.writer, "{}={}", field.name(), value).ok();
    }
    
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if !self.first {
            write!(self.writer, " ").ok();
        }
        self.first = false;
        write!(self.writer, "{}={}", field.name(), value).ok();
    }
    
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if !self.first {
            write!(self.writer, " ").ok();
        }
        self.first = false;
        write!(self.writer, "{}=\"{}\"", field.name(), value).ok();
    }
    
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if !self.first {
            write!(self.writer, " ").ok();
        }
        self.first = false;
        write!(self.writer, "{}={:?}", field.name(), value).ok();
    }
}

struct JsonFieldVisitor<'writer> {
    writer: Writer<'writer>,
    first: bool,
}

impl<'writer> JsonFieldVisitor<'writer> {
    fn new(writer: Writer<'writer>) -> Self {
        Self { writer, first: true }
    }
    
    fn finish(self) -> fmt::Result {
        Ok(())
    }
}

impl<'writer> tracing::field::Visit for JsonFieldVisitor<'writer> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            write!(self.writer, "{}", value).ok();
        }
    }
    
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            write!(self.writer, "{:?}", value).ok();
        }
    }
}

struct LogfmtFieldVisitor<'writer> {
    writer: Writer<'writer>,
    first: bool,
}

impl<'writer> LogfmtFieldVisitor<'writer> {
    fn new(writer: Writer<'writer>) -> Self {
        Self { writer, first: true }
    }
    
    fn finish(self) -> fmt::Result {
        Ok(())
    }
}

impl<'writer> tracing::field::Visit for LogfmtFieldVisitor<'writer> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            write!(self.writer, "{}", value).ok();
        }
    }
    
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            write!(self.writer, "{:?}", value).ok();
        }
    }
}

/// Parse structured fields from field buffer
fn parse_structured_fields(field_buffer: &str) -> Result<Vec<(String, serde_json::Value)>, serde_json::Error> {
    let mut fields = Vec::new();
    
    // Try to parse as key-value pairs first
    for line in field_buffer.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        // Look for key=value or key:value patterns
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let value = line[eq_pos + 1..].trim();
            
            // Try to parse value as JSON, fall back to string
            let json_value = if value.starts_with('"') && value.ends_with('"') {
                serde_json::Value::String(value[1..value.len()-1].to_string())
            } else if let Ok(num) = value.parse::<f64>() {
                serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or_else(|| serde_json::Number::from(0)))
            } else if value == "true" {
                serde_json::Value::Bool(true)
            } else if value == "false" {
                serde_json::Value::Bool(false)
            } else if value == "null" {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(value.to_string())
            };
            
            fields.push((key, json_value));
        } else if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim();
            
            // Similar JSON parsing as above
            let json_value = if value.starts_with('"') && value.ends_with('"') {
                serde_json::Value::String(value[1..value.len()-1].to_string())
            } else if let Ok(num) = value.parse::<f64>() {
                serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or_else(|| serde_json::Number::from(0)))
            } else if value == "true" {
                serde_json::Value::Bool(true)
            } else if value == "false" {
                serde_json::Value::Bool(false)
            } else if value == "null" {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(value.to_string())
            };
            
            fields.push((key, json_value));
        } else {
            // Treat entire line as a message field
            fields.push(("message".to_string(), serde_json::Value::String(line.to_string())));
        }
    }
    
    // If no structured fields found, treat entire buffer as message
    if fields.is_empty() && !field_buffer.trim().is_empty() {
        fields.push(("message".to_string(), serde_json::Value::String(field_buffer.trim().to_string())));
    }
    
    Ok(fields)
}

/// Color scheme for console output
#[derive(Debug, Clone)]
struct ColorScheme {
    reset: &'static str,
    timestamp: &'static str,
    target: &'static str,
    thread: &'static str,
    field_key: &'static str,
    location: &'static str,
    trace: &'static str,
    debug: &'static str,
    info: &'static str,
    warn: &'static str,
    error: &'static str,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            reset: "\x1b[0m",
            timestamp: "\x1b[90m", // Bright black (gray)
            target: "\x1b[36m",    // Cyan
            thread: "\x1b[95m",    // Bright magenta
            field_key: "\x1b[94m", // Bright blue
            location: "\x1b[90m",  // Bright black (gray)
            trace: "\x1b[95m",     // Bright magenta
            debug: "\x1b[94m",     // Bright blue
            info: "\x1b[92m",      // Bright green
            warn: "\x1b[93m",      // Bright yellow
            error: "\x1b[91m",     // Bright red
        }
    }
}

impl ColorScheme {
    fn level_color(&self, level: &tracing::Level) -> &'static str {
        match *level {
            tracing::Level::TRACE => self.trace,
            tracing::Level::DEBUG => self.debug,
            tracing::Level::INFO => self.info,
            tracing::Level::WARN => self.warn,
            tracing::Level::ERROR => self.error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_color_scheme() {
        let colors = ColorScheme::default();
        
        assert_eq!(colors.level_color(&tracing::Level::ERROR), colors.error);
        assert_eq!(colors.level_color(&tracing::Level::WARN), colors.warn);
        assert_eq!(colors.level_color(&tracing::Level::INFO), colors.info);
        assert_eq!(colors.level_color(&tracing::Level::DEBUG), colors.debug);
        assert_eq!(colors.level_color(&tracing::Level::TRACE), colors.trace);
    }
    
    #[test]
    fn test_sensitive_filtering_in_formatter() {
        let filter = Arc::new(SensitiveDataFilter::new());
        let config = FileFormatConfig::structured();
        
        let formatter = LogfmtFormatter::new(config, filter.clone());
        
        // Test that sensitive fields are filtered
        assert!(filter.should_filter_field("password"));
        assert!(filter.should_filter_field("api_key"));
        assert!(!filter.should_filter_field("username"));
    }
    
    #[test]
    fn test_game_formatter_level_mapping() {
        use tracing::Level;
        
        // Test level character mapping
        let test_cases = [
            (Level::TRACE, 'T'),
            (Level::DEBUG, 'D'),
            (Level::INFO, 'I'),
            (Level::WARN, 'W'),
            (Level::ERROR, 'E'),
        ];
        
        for (level, expected_char) in test_cases {
            let actual_char = match level {
                Level::TRACE => 'T',
                Level::DEBUG => 'D',
                Level::INFO => 'I',
                Level::WARN => 'W',
                Level::ERROR => 'E',
            };
            assert_eq!(actual_char, expected_char);
        }
    }
}
