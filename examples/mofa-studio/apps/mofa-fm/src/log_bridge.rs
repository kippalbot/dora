//! Log Bridge - Forwards Rust log messages to the system log panel
//!
//! This module sets up a custom logger that captures all log messages
//! and makes them available to the UI via a channel.

use crossbeam_channel::{Receiver, Sender, bounded};
use log::{Level, LevelFilter, Log, Metadata, Record};
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global log channel sender
static LOG_SENDER: OnceCell<Sender<LogMessage>> = OnceCell::new();

/// Global log channel receiver
static LOG_RECEIVER: OnceCell<Receiver<LogMessage>> = OnceCell::new();

/// Flag to check if logger is initialized
static LOGGER_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// A log message captured from the log crate
#[derive(Clone, Debug)]
pub struct LogMessage {
    pub level: Level,
    pub target: String,
    pub message: String,
}

impl LogMessage {
    /// Format as a log entry string for the system log panel
    pub fn format(&self) -> String {
        let level_str = match self.level {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        };

        // Extract a short module name from target
        let module = self.target
            .split("::")
            .last()
            .unwrap_or(&self.target);

        format!("[{}] [{}] {}", level_str, module, self.message)
    }
}

/// Custom logger that forwards to the log channel
struct BridgeLogger;

impl Log for BridgeLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // Filter out noisy modules
        let target = metadata.target();
        if target.starts_with("wgpu") ||
           target.starts_with("naga") ||
           target.starts_with("winit") ||
           target.starts_with("makepad_platform") {
            return metadata.level() <= Level::Warn;
        }
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        if let Some(sender) = LOG_SENDER.get() {
            let msg = LogMessage {
                level: record.level(),
                target: record.target().to_string(),
                message: format!("{}", record.args()),
            };
            // Non-blocking send - drop if channel is full
            let _ = sender.try_send(msg);
        }
    }

    fn flush(&self) {}
}

/// Initialize the log bridge
///
/// This should be called once at app startup. It sets up the custom logger
/// and creates the channel for log messages.
pub fn init() {
    if LOGGER_INITIALIZED.swap(true, Ordering::SeqCst) {
        return; // Already initialized
    }

    // Create bounded channel (1000 messages max)
    let (tx, rx) = bounded(1000);

    let _ = LOG_SENDER.set(tx);
    let _ = LOG_RECEIVER.set(rx);

    // Set up the logger
    let _ = log::set_logger(&BRIDGE_LOGGER);
    log::set_max_level(LevelFilter::Debug);
}

/// Static logger instance
static BRIDGE_LOGGER: BridgeLogger = BridgeLogger;

/// Poll for new log messages (non-blocking)
///
/// Returns all pending log messages since the last poll.
pub fn poll_logs() -> Vec<LogMessage> {
    let mut logs = Vec::new();

    if let Some(receiver) = LOG_RECEIVER.get() {
        while let Ok(msg) = receiver.try_recv() {
            logs.push(msg);
        }
    }

    logs
}

/// Get the log receiver for direct access
pub fn receiver() -> Option<&'static Receiver<LogMessage>> {
    LOG_RECEIVER.get()
}
