use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;

use quote_common::QuoteError;

/// Command line arguments for the quote client.
#[derive(Debug, Parser)]
#[command(author, version, about = "Quote streaming client", long_about = None)]
pub struct CliArgs {
    /// TCP address of the quote server (e.g., 127.0.0.1:8080)
    #[arg(long = "server-addr")]
    pub server_addr: String,

    /// Local UDP port to bind for receiving quotes
    #[arg(long = "udp-port")]
    pub udp_port: u16,

    /// Path to file containing ticker symbols (one per line)
    #[arg(long = "tickers-file")]
    pub tickers_file: PathBuf,
}

/// Parse command line arguments.
pub fn parse() -> CliArgs {
    CliArgs::parse()
}

/// Load ticker symbols from the provided file, normalizing to uppercase.
pub fn load_tickers(path: &Path) -> Result<Vec<String>, QuoteError> {
    let contents = fs::read_to_string(path).map_err(|err| {
        quote_common::quote_error!(
            IoError,
            err,
            "failed to read ticker file '{}'",
            path.display()
        )
    })?;
    let mut tickers = Vec::new();
    for line in contents.lines() {
        let ticker = line.trim();
        if ticker.is_empty() {
            continue;
        }
        tickers.push(ticker.to_uppercase());
    }

    if tickers.is_empty() {
        return Err(quote_common::quote_error!(
            ConfigError,
            "ticker file '{}' contained no symbols",
            path.display()
        ));
    }

    Ok(tickers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        path.push(format!("tickers_test_{nanos}.txt"));
        path
    }

    #[test]
    fn test_load_tickers_filters_and_uppercases() {
        let path = unique_temp_path();
        let mut file = fs::File::create(&path).expect("create temp file");
        file.write_all(b"aapl\n").unwrap();
        file.write_all(b"  msft \n").unwrap();
        file.write_all(b"\n").unwrap();
        file.write_all(b"TsLa\n").unwrap();
        drop(file);

        let tickers = load_tickers(&path).expect("load tickers");
        assert_eq!(tickers, vec!["AAPL", "MSFT", "TSLA"]);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_load_tickers_empty_file() {
        let path = unique_temp_path();
        {
            let _file = fs::File::create(&path).expect("create temp file");
            // File is empty, will be closed when dropped
        }

        let err = load_tickers(&path).expect_err("should fail");
        assert!(matches!(err, QuoteError::ConfigError { .. }));

        fs::remove_file(path).unwrap();
    }
}
