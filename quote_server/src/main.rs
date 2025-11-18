mod config;
mod generator;
mod tcp_handler;
mod udp_streamer;

use std::path::Path;
use std::time::Duration;

use crossbeam::channel;
use log::info;

use quote_common::QuoteError;

use config::{load_config, load_tickers};
use generator::start_generator;
use tcp_handler::{StreamRequest, start_tcp_server};
use udp_streamer::{UdpCommand, start_udp_streamer};

fn main() {
    env_logger::init();

    // Set up Ctrl+C handler for graceful shutdown
    let shutdown_signal_rx =
        quote_common::setup_shutdown_signal().expect("Failed to setup shutdown signal handler");

    if let Err(err) = run(shutdown_signal_rx) {
        quote_common::log_error!(err, "Server error");
        std::process::exit(1);
    }

    info!("Server shutdown complete");
}

fn run(shutdown_signal_rx: crossbeam::channel::Receiver<()>) -> Result<(), QuoteError> {
    let config = load_config(Path::new("server_config.toml"))?;

    info!("Loaded configuration:");
    info!("  TCP address: {}", config.tcp_addr);
    info!("  Tickers file: {}", config.tickers_file);
    info!("  Quote rate: {}ms", config.quote_rate_ms);
    info!("  Keepalive timeout: {}s", config.keepalive_timeout_secs);
    info!("  Initial prices: {} tickers", config.initial_prices.len());

    let tickers = load_tickers(Path::new(&config.tickers_file))?;
    info!("Loaded {} tickers from file", tickers.len());

    let (quote_rx, generator_handle) = start_generator(
        tickers,
        config.initial_prices.clone(),
        Some(config.quote_rate_ms),
    )?;

    let keepalive_timeout = Duration::from_secs(config.keepalive_timeout_secs);
    let server_tcp_addr: std::net::SocketAddr = config.tcp_addr.parse().map_err(|err| {
        quote_common::quote_error!(
            ConfigError,
            "invalid TCP address '{}': {}",
            config.tcp_addr,
            err
        )
    })?;
    let (dispatcher_tx, dispatcher_handle) =
        start_udp_streamer(quote_rx, keepalive_timeout, server_tcp_addr)?;

    let (request_tx, request_rx) = channel::unbounded::<StreamRequest>();
    // FIX: Store shutdown_tx instead of dropping it immediately with underscore
    let (shutdown_tx, tcp_handle) = start_tcp_server(&config.tcp_addr, request_tx.clone())?;

    // Drop main thread's sender - TCP thread now owns the only active sender
    // This allows the recv loop to exit when TCP thread finishes
    drop(request_tx);

    // Process requests until shutdown signal or TCP server exits
    loop {
        crossbeam::channel::select! {
            recv(request_rx) -> result => match result {
                Ok(request) => {
                    log_stream_request(&request);
                    dispatcher_tx
                        .send(UdpCommand::AddClient(request))
                        .map_err(|err| {
                            quote_common::quote_error!(NetworkError, "failed to register UDP client: {}", err)
                        })?;
                }
                Err(_) => {
                    info!("TCP server has stopped sending requests");
                    break;
                }
            },
            recv(shutdown_signal_rx) -> _ => {
                info!("Shutdown signal received, stopping server...");
                break;
            }
        }
    }

    // Signal TCP server to shutdown
    info!("Signaling TCP server shutdown...");
    let _ = shutdown_tx.send(());  // Send shutdown signal
    drop(shutdown_tx);

    // Wait for TCP server thread to finish
    tcp_handle
        .join()
        .map_err(|_| quote_common::quote_error!(NetworkError, "tcp server thread panicked"))?;

    dispatcher_tx.send(UdpCommand::Shutdown).map_err(|err| {
        quote_common::quote_error!(NetworkError, "failed to stop UDP dispatcher: {}", err)
    })?;
    dispatcher_handle
        .join()
        .map_err(|_| quote_common::quote_error!(NetworkError, "udp dispatcher thread panicked"))?;

    generator_handle
        .join()
        .map_err(|_| quote_common::quote_error!(NetworkError, "generator thread panicked"))?;

    Ok(())
}

fn log_stream_request(request: &StreamRequest) {
    info!(
        "Client requested STREAM to {} for [{}]",
        request.udp_addr,
        request.tickers.join(",")
    );
}
