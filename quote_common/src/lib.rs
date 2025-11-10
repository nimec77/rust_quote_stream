//! Shared types and utilities for the quote streaming system.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

/// Unified error type for the quote streaming system.
#[derive(Debug, Error)]
pub enum QuoteError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_quote_error_from_io_error() {
        let io_err = io::Error::other("network failure");
        let quote_err = QuoteError::from(io_err);

        assert!(matches!(quote_err, QuoteError::IoError(_)));
    }
}
