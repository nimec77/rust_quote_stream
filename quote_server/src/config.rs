use std::collections::HashMap;
use std::fs;
use std::path::Path;

use quote_common::{DEFAULT_KEEPALIVE_TIMEOUT_SECS, DEFAULT_QUOTE_RATE_MS, QuoteError};

/// Server configuration loaded from TOML file.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub tcp_addr: String,
    pub tickers_file: String,
    pub quote_rate_ms: u64,
    pub keepalive_timeout_secs: u64,
    pub initial_prices: HashMap<String, f64>,
}

/// Load server configuration from a TOML file.
pub fn load_config(path: &Path) -> Result<ServerConfig, QuoteError> {
    let contents = fs::read_to_string(path).map_err(|err| {
        QuoteError::ConfigError(format!(
            "failed to read config file '{}': {}",
            path.display(),
            err
        ))
    })?;

    let parsed: toml::Table = toml::from_str(&contents).map_err(|err| {
        QuoteError::ConfigError(format!(
            "invalid TOML syntax in '{}': {}",
            path.display(),
            err
        ))
    })?;

    let tcp_addr = parsed
        .get("tcp_addr")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            QuoteError::ConfigError(format!(
                "missing required field 'tcp_addr' in '{}'",
                path.display()
            ))
        })?
        .to_string();

    let tickers_file = parsed
        .get("tickers_file")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            QuoteError::ConfigError(format!(
                "missing required field 'tickers_file' in '{}'",
                path.display()
            ))
        })?
        .to_string();

    let quote_rate_ms = parsed
        .get("quote_rate_ms")
        .and_then(|v| v.as_integer())
        .map(|i| i as u64)
        .unwrap_or(DEFAULT_QUOTE_RATE_MS);

    let keepalive_timeout_secs = parsed
        .get("keepalive_timeout_secs")
        .and_then(|v| v.as_integer())
        .map(|i| i as u64)
        .unwrap_or(DEFAULT_KEEPALIVE_TIMEOUT_SECS);

    let initial_prices = parsed
        .get("initial_prices")
        .and_then(|v| v.as_table())
        .map(|tbl| {
            let mut prices = HashMap::new();
            for (ticker, value) in tbl {
                if let Some(price) = value.as_float() {
                    prices.insert(ticker.to_uppercase(), price);
                }
            }
            prices
        })
        .unwrap_or_default();

    Ok(ServerConfig {
        tcp_addr,
        tickers_file,
        quote_rate_ms,
        keepalive_timeout_secs,
        initial_prices,
    })
}

/// Load ticker symbols from a file, normalizing to uppercase.
pub fn load_tickers(path: &Path) -> Result<Vec<String>, QuoteError> {
    let contents = fs::read_to_string(path)?;
    let mut tickers = Vec::new();

    for line in contents.lines() {
        let ticker = line.trim();
        if ticker.is_empty() {
            continue;
        }
        tickers.push(ticker.to_uppercase());
    }

    if tickers.is_empty() {
        return Err(QuoteError::ConfigError(format!(
            "ticker file '{}' contained no symbols",
            path.display()
        )));
    }

    Ok(tickers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(prefix: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        path.push(format!("{}_{nanos}.toml", prefix));
        path
    }

    #[test]
    fn test_load_config_valid() {
        let path = unique_temp_path("config");
        let mut file = fs::File::create(&path).expect("create temp file");
        writeln!(file, "tcp_addr = \"127.0.0.1:8080\"").unwrap();
        writeln!(file, "tickers_file = \"tickers.txt\"").unwrap();
        writeln!(file, "quote_rate_ms = 500").unwrap();
        writeln!(file, "keepalive_timeout_secs = 10").unwrap();
        writeln!(file, "[initial_prices]").unwrap();
        writeln!(file, "AAPL = 150.0").unwrap();
        writeln!(file, "TSLA = 250.5").unwrap();
        drop(file);

        let config = load_config(&path).expect("load config");
        assert_eq!(config.tcp_addr, "127.0.0.1:8080");
        assert_eq!(config.tickers_file, "tickers.txt");
        assert_eq!(config.quote_rate_ms, 500);
        assert_eq!(config.keepalive_timeout_secs, 10);
        assert_eq!(config.initial_prices.get("AAPL"), Some(&150.0));
        assert_eq!(config.initial_prices.get("TSLA"), Some(&250.5));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_load_config_with_defaults() {
        let path = unique_temp_path("config");
        let mut file = fs::File::create(&path).expect("create temp file");
        writeln!(file, "tcp_addr = \"127.0.0.1:8080\"").unwrap();
        writeln!(file, "tickers_file = \"tickers.txt\"").unwrap();
        drop(file);

        let config = load_config(&path).expect("load config");
        assert_eq!(config.quote_rate_ms, DEFAULT_QUOTE_RATE_MS);
        assert_eq!(
            config.keepalive_timeout_secs,
            DEFAULT_KEEPALIVE_TIMEOUT_SECS
        );
        assert!(config.initial_prices.is_empty());

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_load_config_missing_file() {
        let path = Path::new("/nonexistent/config.toml");
        let err = load_config(path).expect_err("should fail");
        assert!(matches!(err, QuoteError::ConfigError(_)));
        assert!(err.to_string().contains("failed to read config file"));
    }

    #[test]
    fn test_load_config_missing_required_field() {
        let path = unique_temp_path("config");
        let mut file = fs::File::create(&path).expect("create temp file");
        writeln!(file, "tcp_addr = \"127.0.0.1:8080\"").unwrap();
        // Missing tickers_file
        drop(file);

        let err = load_config(&path).expect_err("should fail");
        assert!(matches!(err, QuoteError::ConfigError(_)));
        assert!(err.to_string().contains("tickers_file"));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_load_config_invalid_toml() {
        let path = unique_temp_path("config");
        let mut file = fs::File::create(&path).expect("create temp file");
        writeln!(file, "tcp_addr = [invalid").unwrap();
        drop(file);

        let err = load_config(&path).expect_err("should fail");
        assert!(matches!(err, QuoteError::ConfigError(_)));
        assert!(err.to_string().contains("invalid TOML syntax"));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_load_tickers_valid() {
        let path = unique_temp_path("tickers");
        let mut file = fs::File::create(&path).expect("create temp file");
        writeln!(file, "AAPL").unwrap();
        writeln!(file, "  MSFT ").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "tsla").unwrap();
        drop(file);

        let tickers = load_tickers(&path).expect("load tickers");
        assert_eq!(tickers, vec!["AAPL", "MSFT", "TSLA"]);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_load_tickers_empty_file() {
        let path = unique_temp_path("tickers");
        {
            let _file = fs::File::create(&path).expect("create temp file");
            // File is empty
        }

        let err = load_tickers(&path).expect_err("should fail");
        assert!(matches!(err, QuoteError::ConfigError(_)));
        assert!(err.to_string().contains("contained no symbols"));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_load_tickers_missing_file() {
        let path = Path::new("/nonexistent/tickers.txt");
        let err = load_tickers(path).expect_err("should fail");
        assert!(matches!(err, QuoteError::IoError(_)));
    }
}
