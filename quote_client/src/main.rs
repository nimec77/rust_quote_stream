use std::net::{SocketAddr, UdpSocket};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use log::info;

use quote_common::QuoteError;

mod cli;
mod tcp_client;
mod udp_receiver;

use cli::{load_tickers, parse};
use tcp_client::send_stream_command;
use udp_receiver::{spawn_listener, spawn_ping_thread};

fn main() {
    env_logger::init();

    if let Err(err) = run() {
        quote_common::log_error!(err, "Client exited with error");
        std::process::exit(1);
    }
}

fn run() -> Result<(), QuoteError> {
    let args = parse();

    let tickers = load_tickers(&args.tickers_file)?;
    let server_addr: SocketAddr = args.server_addr.parse().map_err(|err| {
        quote_common::quote_error!(
            ConfigError,
            "invalid server address '{}': {}",
            args.server_addr,
            err
        )
    })?;

    let socket = UdpSocket::bind(("0.0.0.0", args.udp_port)).map_err(|err| {
        quote_common::quote_error!(NetworkError, "failed to bind UDP socket: {}", err)
    })?;
    let local_addr = socket.local_addr().map_err(|err| {
        quote_common::quote_error!(NetworkError, "failed to read UDP socket address: {}", err)
    })?;

    let advertised_udp_addr = format!("{}:{}", server_addr.ip(), local_addr.port());
    info!(
        "Bound UDP listener on {} (advertising to server as {})",
        local_addr, advertised_udp_addr
    );

    send_stream_command(&args.server_addr, &advertised_udp_addr, &tickers)?;

    let shutdown = Arc::new(AtomicBool::new(false));

    // Clone socket for ping thread before moving original to listener
    let ping_socket = socket.try_clone().map_err(|err| {
        quote_common::quote_error!(NetworkError, "failed to clone UDP socket: {}", err)
    })?;

    let listener_handle = spawn_listener(socket, Arc::clone(&shutdown))?;
    let ping_handle = spawn_ping_thread(ping_socket, server_addr, Arc::clone(&shutdown))?;

    let (signal_tx, signal_rx) = std::sync::mpsc::channel::<()>();
    ctrlc::set_handler(move || {
        let _ = signal_tx.send(());
    })
    .map_err(|err| {
        quote_common::quote_error!(ConfigError, "failed to install Ctrl+C handler: {}", err)
    })?;

    info!("STREAM established; press Ctrl+C to stop.");
    let _ = signal_rx.recv();

    shutdown.store(true, Ordering::SeqCst);

    // Allow threads to notice shutdown signal.
    std::thread::sleep(Duration::from_millis(200));

    listener_handle
        .join()
        .map_err(|_| quote_common::quote_error!(NetworkError, "UDP listener thread panicked"))?;
    ping_handle
        .join()
        .map_err(|_| quote_common::quote_error!(NetworkError, "ping thread panicked"))?;

    info!("Client shut down cleanly.");

    Ok(())
}
