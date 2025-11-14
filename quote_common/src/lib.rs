//! Shared types and utilities for the quote streaming system.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::backtrace::Backtrace;

/// Default quote generation interval in milliseconds.
pub const DEFAULT_QUOTE_RATE_MS: u64 = 1_000;
/// Default keepalive timeout in seconds on the server.
pub const DEFAULT_KEEPALIVE_TIMEOUT_SECS: u64 = 5;
/// Interval in seconds for client PING messages.
pub const PING_INTERVAL_SECS: u64 = 2;
/// Default initial price when configuration omits a ticker.
pub const DEFAULT_INITIAL_PRICE: f64 = 100.0;
/// Popular tickers receive higher default volume ranges.
pub const POPULAR_TICKERS: &[&str] = &["AAPL", "MSFT", "TSLA"];

/// Representation of a stock quote transmitted over UDP.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StockQuote {
    /// Uppercase ticker symbol.
    pub ticker: String,
    /// Last traded price (two decimal places when formatted for output).
    pub price: f64,
    /// Trade volume in shares.
    pub volume: u32,
    /// Unix timestamp in milliseconds (UTC).
    pub timestamp: i64,
}

impl StockQuote {
    /// Convenience constructor producing a quote with current UTC timestamp.
    pub fn new(ticker: impl Into<String>, price: f64, volume: u32) -> Self {
        Self {
            ticker: ticker.into(),
            price,
            volume,
            timestamp: Utc::now().timestamp_millis(),
        }
    }
}

/// Location information for where an error occurred.
#[derive(Debug, Clone)]
pub struct ErrorLocation {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

/// Unified error type for the quote streaming system with location tracking.
#[derive(Debug)]
pub enum QuoteError {
    IoError {
        source: std::io::Error,
        context: String,
        location: ErrorLocation,
        backtrace: Backtrace,
    },
    ParseError {
        message: String,
        location: ErrorLocation,
        backtrace: Backtrace,
    },
    NetworkError {
        message: String,
        location: ErrorLocation,
        backtrace: Backtrace,
    },
    SerializationError {
        message: String,
        location: ErrorLocation,
        backtrace: Backtrace,
    },
    InvalidCommand {
        message: String,
        location: ErrorLocation,
        backtrace: Backtrace,
    },
    ConfigError {
        message: String,
        location: ErrorLocation,
        backtrace: Backtrace,
    },
}

impl std::fmt::Display for QuoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuoteError::IoError {
                source,
                context,
                location,
                ..
            } => {
                write!(
                    f,
                    "I/O error: {} ({}:{}:{}): {}",
                    context, location.file, location.line, location.column, source
                )
            }
            QuoteError::ParseError {
                message, location, ..
            } => {
                write!(
                    f,
                    "Parse error: {} ({}:{}:{})",
                    message, location.file, location.line, location.column
                )
            }
            QuoteError::NetworkError {
                message, location, ..
            } => {
                write!(
                    f,
                    "Network error: {} ({}:{}:{})",
                    message, location.file, location.line, location.column
                )
            }
            QuoteError::SerializationError {
                message, location, ..
            } => {
                write!(
                    f,
                    "Serialization error: {} ({}:{}:{})",
                    message, location.file, location.line, location.column
                )
            }
            QuoteError::InvalidCommand {
                message, location, ..
            } => {
                write!(
                    f,
                    "Invalid command: {} ({}:{}:{})",
                    message, location.file, location.line, location.column
                )
            }
            QuoteError::ConfigError {
                message, location, ..
            } => {
                write!(
                    f,
                    "Configuration error: {} ({}:{}:{})",
                    message, location.file, location.line, location.column
                )
            }
        }
    }
}

impl std::error::Error for QuoteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QuoteError::IoError { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Creates a QuoteError with automatic location tracking.
///
/// # Usage
///
/// ```ignore
/// // For IoError with source error
/// quote_error!(IoError, io_err, "Failed to read config file")
///
/// // For other error types with just a message
/// quote_error!(ConfigError, "Missing required field: {}", field_name)
/// quote_error!(ParseError, "Invalid ticker format")
/// ```
#[macro_export]
macro_rules! quote_error {
    // For IoError with source
    (IoError, $source:expr, $($arg:tt)*) => {
        $crate::QuoteError::IoError {
            source: $source,
            context: format!($($arg)*),
            location: $crate::ErrorLocation {
                file: file!(),
                line: line!(),
                column: column!(),
            },
            backtrace: std::backtrace::Backtrace::capture(),
        }
    };

    // For ParseError with message
    (ParseError, $($arg:tt)*) => {
        $crate::QuoteError::ParseError {
            message: format!($($arg)*),
            location: $crate::ErrorLocation {
                file: file!(),
                line: line!(),
                column: column!(),
            },
            backtrace: std::backtrace::Backtrace::capture(),
        }
    };

    // For NetworkError with message
    (NetworkError, $($arg:tt)*) => {
        $crate::QuoteError::NetworkError {
            message: format!($($arg)*),
            location: $crate::ErrorLocation {
                file: file!(),
                line: line!(),
                column: column!(),
            },
            backtrace: std::backtrace::Backtrace::capture(),
        }
    };

    // For SerializationError with message
    (SerializationError, $($arg:tt)*) => {
        $crate::QuoteError::SerializationError {
            message: format!($($arg)*),
            location: $crate::ErrorLocation {
                file: file!(),
                line: line!(),
                column: column!(),
            },
            backtrace: std::backtrace::Backtrace::capture(),
        }
    };

    // For InvalidCommand with message
    (InvalidCommand, $($arg:tt)*) => {
        $crate::QuoteError::InvalidCommand {
            message: format!($($arg)*),
            location: $crate::ErrorLocation {
                file: file!(),
                line: line!(),
                column: column!(),
            },
            backtrace: std::backtrace::Backtrace::capture(),
        }
    };

    // For ConfigError with message
    (ConfigError, $($arg:tt)*) => {
        $crate::QuoteError::ConfigError {
            message: format!($($arg)*),
            location: $crate::ErrorLocation {
                file: file!(),
                line: line!(),
                column: column!(),
            },
            backtrace: std::backtrace::Backtrace::capture(),
        }
    };
}

/// Logs an error with location and backtrace information.
///
/// # Usage
///
/// ```ignore
/// if let Err(e) = risky_operation() {
///     log_error!(e, "Failed to perform operation");
///     return Err(e);
/// }
/// ```
#[macro_export]
macro_rules! log_error {
    ($err:expr, $($arg:tt)*) => {{
        use log::error;
        let location = match &$err {
            $crate::QuoteError::IoError { location, .. } => location,
            $crate::QuoteError::ParseError { location, .. } => location,
            $crate::QuoteError::NetworkError { location, .. } => location,
            $crate::QuoteError::SerializationError { location, .. } => location,
            $crate::QuoteError::InvalidCommand { location, .. } => location,
            $crate::QuoteError::ConfigError { location, .. } => location,
        };

        let backtrace = match &$err {
            $crate::QuoteError::IoError { backtrace, .. } => backtrace,
            $crate::QuoteError::ParseError { backtrace, .. } => backtrace,
            $crate::QuoteError::NetworkError { backtrace, .. } => backtrace,
            $crate::QuoteError::SerializationError { backtrace, .. } => backtrace,
            $crate::QuoteError::InvalidCommand { backtrace, .. } => backtrace,
            $crate::QuoteError::ConfigError { backtrace, .. } => backtrace,
        };

        error!("{}", format!($($arg)*));
        error!("  at {}:{}:{}", location.file, location.line, location.column);
        error!("  Error: {}", $err);

        // Only print backtrace if RUST_BACKTRACE is set
        if std::env::var("RUST_BACKTRACE").is_ok() {
            error!("  Stack trace:\n{}", backtrace);
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::io;

    #[test]
    fn test_stock_quote_json_roundtrip() {
        let quote = StockQuote {
            ticker: "AAPL".to_string(),
            price: 150.25,
            volume: 3_500,
            timestamp: 1_699_564_800_000,
        };

        let json = serde_json::to_string(&quote).expect("serialize quote");
        assert_eq!(
            json,
            r#"{"ticker":"AAPL","price":150.25,"volume":3500,"timestamp":1699564800000}"#
        );

        let restored: StockQuote = serde_json::from_str(&json).expect("deserialize quote");
        assert_eq!(restored, quote);
    }

    #[test]
    fn test_error_location_capture() {
        let io_err = io::Error::other("network failure");
        let quote_err = quote_error!(IoError, io_err, "Failed to connect");

        match quote_err {
            QuoteError::IoError { location, .. } => {
                assert!(location.file.contains("lib.rs"));
                assert!(location.line > 0);
                assert!(location.column > 0);
            }
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_error_display_includes_location() {
        let err = quote_error!(ParseError, "Invalid ticker format");
        let display = format!("{}", err);

        assert!(display.contains("Parse error"));
        assert!(display.contains("Invalid ticker format"));
        assert!(display.contains("lib.rs"));
    }

    #[test]
    fn test_config_error_creation() {
        let err = quote_error!(ConfigError, "Missing field: {}", "tcp_addr");

        match err {
            QuoteError::ConfigError { message, .. } => {
                assert_eq!(message, "Missing field: tcp_addr");
            }
            _ => panic!("Expected ConfigError variant"),
        }
    }

    #[test]
    fn test_serialization_error_creation() {
        let err = quote_error!(SerializationError, "Failed to serialize quote");

        match err {
            QuoteError::SerializationError { message, .. } => {
                assert_eq!(message, "Failed to serialize quote");
            }
            _ => panic!("Expected SerializationError variant"),
        }
    }

    #[test]
    fn test_network_error_creation() {
        let err = quote_error!(NetworkError, "Connection timeout");

        match err {
            QuoteError::NetworkError { message, .. } => {
                assert_eq!(message, "Connection timeout");
            }
            _ => panic!("Expected NetworkError variant"),
        }
    }

    #[test]
    fn test_invalid_command_error_creation() {
        let err = quote_error!(InvalidCommand, "Unknown command: STOP");

        match err {
            QuoteError::InvalidCommand { message, .. } => {
                assert_eq!(message, "Unknown command: STOP");
            }
            _ => panic!("Expected InvalidCommand variant"),
        }
    }

    #[test]
    fn test_error_source_for_io_error() {
        let io_err = io::Error::other("disk full");
        let quote_err = quote_error!(IoError, io_err, "Write failed");

        assert!(quote_err.source().is_some());
    }

    #[test]
    fn test_error_source_for_other_errors() {
        let err = quote_error!(ParseError, "Bad format");
        assert!(err.source().is_none());
    }
}
