# rust_quote_stream

A real-time stock quote streaming system built with Rust. The system consists of a server that generates and streams stock quotes, and clients that connect to receive filtered quote data.

## Architecture

The project is organized as a Cargo workspace with three crates:

- **`quote_common`** – Shared data types (`StockQuote`, `QuoteError`) and constants
- **`quote_server`** – Quote generator and TCP/UDP streaming server
- **`quote_client`** – CLI client for consuming streamed quotes

## Building

```bash
# Build all crates
cargo build

# Build release binaries
cargo build --release
```

## Usage

### Server

The server reads configuration from `server_config.toml` in the current directory.

```bash
# Start the server
cargo run --bin quote_server

# With custom log level
RUST_LOG=debug cargo run --bin quote_server
RUST_LOG=info cargo run --bin quote_server  # default
RUST_LOG=warn cargo run --bin quote_server
RUST_LOG=error cargo run --bin quote_server
```

The server will:
1. Load configuration from `server_config.toml`
2. Read ticker symbols from the configured tickers file
3. Start generating quotes at the configured rate
4. Listen for TCP connections on the configured address
5. Stream quotes to connected clients via UDP

### Client

The client connects to the server and receives quotes for specified tickers.

```bash
# Connect to server
cargo run --bin quote_client -- \
  --server-addr 127.0.0.1:8080 \
  --udp-port 34254 \
  --tickers-file tickers.txt

# With custom log level
RUST_LOG=info cargo run --bin quote_client -- \
  --server-addr 127.0.0.1:8080 \
  --udp-port 34254 \
  --tickers-file tickers.txt
```

**Client Arguments:**
- `--server-addr`: TCP address of the quote server (e.g., `127.0.0.1:8080`)
- `--udp-port`: Local UDP port to bind for receiving quotes (e.g., `34254`)
- `--tickers-file`: Path to file containing ticker symbols (one per line)

The client will:
1. Parse command-line arguments
2. Load ticker symbols from the specified file
3. Bind a UDP socket on the specified port
4. Connect to the server via TCP
5. Send a STREAM command with the UDP address and ticker list
6. Receive and log quotes matching the requested tickers
7. Send PING messages every 2 seconds to maintain connection
8. Gracefully shutdown on Ctrl+C

## Configuration

### Server Configuration (`server_config.toml`)

The server requires a `server_config.toml` file in the current directory:

```toml
# TCP address to bind the server listener
tcp_addr = "127.0.0.1:8080"

# Path to file containing ticker symbols (one per line)
tickers_file = "tickers.txt"

# Quote generation interval in milliseconds (optional, default: 1000)
quote_rate_ms = 1000

# Keepalive timeout in seconds (optional, default: 5)
keepalive_timeout_secs = 5

# Initial prices for tickers (optional)
[initial_prices]
AAPL = 150.0
GOOGL = 140.0
TSLA = 250.0
MSFT = 380.0
NVDA = 500.0
AMZN = 180.0
META = 350.0
JPM = 155.0
```

**Required fields:**
- `tcp_addr`: TCP address for the server listener
- `tickers_file`: Path to ticker symbols file

**Optional fields:**
- `quote_rate_ms`: Milliseconds between quote generation cycles (default: 1000)
- `keepalive_timeout_secs`: Seconds before disconnecting inactive clients (default: 5)
- `[initial_prices]`: Initial prices for tickers (default: 100.0 for unspecified tickers)

### Ticker Files

Ticker files contain one ticker symbol per line. Empty lines and whitespace are ignored. Symbols are automatically converted to uppercase.

**Example `tickers.txt`:**
```
AAPL
GOOGL
TSLA
MSFT
NVDA
AMZN
META
JPM
```

## Protocol

### TCP Control Channel

**Client → Server:**
```
STREAM udp://<ip>:<port> <ticker1>,<ticker2>,...
```

**Server → Client:**
```
OK
```
or
```
ERR <message>
```

### UDP Data Channel

**Server → Client:**
JSON-serialized `StockQuote` objects:
```json
{"ticker":"AAPL","price":150.25,"volume":3500,"timestamp":1699564800000}
```

**Client → Server:**
Plain text keep-alive messages:
```
PING
```

## Features

- **Real-time quote generation**: Random walk price simulation with configurable rate
- **Multi-client support**: Each client receives quotes in a dedicated thread
- **Ticker filtering**: Clients only receive quotes for requested tickers
- **Keep-alive mechanism**: Server detects disconnected clients and cleans up resources
- **Graceful shutdown**: Both server and client handle Ctrl+C cleanly
- **Comprehensive logging**: Structured logging with configurable levels
- **Error handling**: All operations return `Result` types with clear error messages

## Testing

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test --package quote_server
cargo test --package quote_client
cargo test --package quote_common

# Run with output
cargo test -- --nocapture
```

## Code Quality

```bash
# Format code
cargo fmt

# Run clippy linter
cargo clippy --workspace

# Build with warnings as errors
cargo clippy --workspace -- -D warnings
```

## Development

Development follows the staged plan outlined in `doc/tasklist.md`. The project architecture and design decisions are documented in `vision.md`. Coding conventions are defined in `conventions.md`.

## License

This is a prototype project for testing and demonstration purposes.
