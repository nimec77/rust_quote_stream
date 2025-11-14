# Technical Vision: Quote Streaming System

> **Project Goal:** Build the simplest possible real-time stock quote streaming system to test our idea, following KISS principles. No overengineering, only essentials.

---

## 1. Technologies

### Core Stack:
- **Rust** (stable channel)
- **Standard Library** - TCP/UDP networking (`std::net`)
- **clap** v4 - CLI argument parsing
- **serde** + **serde_json** - JSON serialization
- **crossbeam** - MPMC channels for quote distribution
- **rand** - Random number generation for price simulation
- **chrono** - Timestamp handling
- **log** + **env_logger** - Logging infrastructure
- **toml** - Configuration file parsing (server only)

### Development Tools:
- **cargo** - Build system and dependency management
- **rustfmt** - Code formatting
- **clippy** - Linting

**Philosophy:** Minimal dependencies, maximum simplicity. No async runtime, no databases, no web frameworks.

---

## 2. Development Principles

1. **KISS (Keep It Simple, Stupid)**
   - No premature optimization
   - Straightforward implementations over clever solutions
   - Clear code over compact code

2. **Fail Fast**
   - Use `Result<T, E>` for error handling
   - Propagate errors with `?` operator
   - Avoid `unwrap()` in production code paths

3. **Explicit over Implicit**
   - Clear function signatures
   - Minimal use of macros
   - Explicit error types

4. **Single Responsibility**
   - Each module handles one concern
   - Functions do one thing well
   - Separate concerns (networking, data generation, parsing)

5. **Test Core Logic**
   - Write unit tests for all main functions
   - Test data generation, serialization, parsing
   - Manual/integration tests for networking

6. **Clean Resource Management**
   - Ensure proper thread termination
   - Close sockets explicitly
   - No resource leaks (connections, file handles)

7. **No Premature Abstraction**
   - Build for current requirements only
   - Refactor when patterns emerge

---

## 3. Project Structure

```
rust_quote_stream/
├── Cargo.toml              # Workspace definition
├── README.md               # Setup and usage instructions
├── vision.md               # This file
├── idea.md                 # Project requirements
├── tickers.txt             # Example ticker list
├── server_config.toml      # Server configuration
│
├── quote_common/           # Shared library crate
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs          # StockQuote struct, constants, serialization
│
├── quote_server/           # Server binary crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # Entry point
│       ├── generator.rs    # Quote generation logic (with tests)
│       ├── tcp_handler.rs  # TCP command handling (with tests)
│       └── udp_streamer.rs # UDP streaming & keep-alive (with tests)
│
└── quote_client/           # Client binary crate
    ├── Cargo.toml
    └── src/
        ├── main.rs         # Entry point
        ├── cli.rs          # CLI argument parsing
        ├── tcp_client.rs   # TCP connection & commands (with tests)
        └── udp_receiver.rs # UDP data reception & ping thread (with tests)
```

**Key Points:**
- 3 crates: `quote_common` (lib), `quote_server` (bin), `quote_client` (bin)
- Shared types/logic in `quote_common`
- Tests inline in code modules (`#[cfg(test)]`)
- CLI parsing in separate `cli.rs` module

---

## 4. Architecture

### Server Architecture:

```
Main Thread (TCP Listener)
    ↓
    ├─> Generator Thread (starts immediately)
    │   ├─> Generates quotes continuously (configurable rate, default: 1/second)
    │   └─> Broadcasts to MPMC Channel
    │
    └─> Per-Client Threads (created on first STREAM command)
        ├─> Receives from MPMC channel
        ├─> Filters by client's tickers
        ├─> Sends via UDP to client
        └─> Monitors keep-alive (PING) via UDP
            └─> Timeout (5s) → Thread terminates
```

### Client Architecture:

```
Main Thread
    ├─> Connects TCP, sends STREAM command
    ├─> Spawns UDP Receiver Thread
    │   └─> Receives quotes, logs to output
    │
    └─> Spawns Ping Thread
        └─> Sends PING every 2s via UDP
```

### Communication Protocols:

**TCP (Control Channel):**
- Client → Server: `STREAM udp://<ip>:<port> <ticker1>,<ticker2>,...`
- Server → Client: `OK` or `ERR <message>`
- Only first STREAM command per connection is accepted

**UDP (Data Channel):**
- Server → Client: JSON-serialized `StockQuote`
- Client → Server: `PING` message (keep-alive)

### Concurrency Model:
- **MPMC Channel**: Generator broadcasts to all client threads
- **Thread-per-Client**: Isolated client handling, automatic cleanup on disconnect
- **Keep-alive**: 5-second timeout, cleanup on PING absence

---

## 5. Data Model

### Core Data Structure:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockQuote {
    pub ticker: String,
    pub price: f64,
    pub volume: u32,
    pub timestamp: i64,  // Unix timestamp (milliseconds)
}
```

### Supporting Types:

```rust
// Server configuration
pub struct ServerConfig {
    pub tcp_addr: String,           // e.g., "127.0.0.1:8080"
    pub tickers_file: String,       // Path to tickers file
    pub quote_rate_ms: u64,         // milliseconds between quotes (default: 1000)
    pub keepalive_timeout_secs: u64, // default: 5
    pub initial_prices: HashMap<String, f64>, // Initial prices per ticker
}

// Client stream request
pub struct StreamRequest {
    pub udp_addr: String,  // e.g., "127.0.0.1:34254"
    pub tickers: Vec<String>,
}
```

### Error Handling:

```rust
use std::backtrace::Backtrace;

#[derive(Debug)]
pub struct ErrorLocation {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

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

// Helper macros for creating errors with automatic location capture
// quote_error!(IoError(io_err), "Context message")
// log_error!(error_value, "High-level operation description")
```

### Constants:

```rust
// In quote_common/src/lib.rs
pub const POPULAR_TICKERS: &[&str] = &["AAPL", "MSFT", "TSLA"];
pub const DEFAULT_QUOTE_RATE_MS: u64 = 1000;
pub const DEFAULT_KEEPALIVE_TIMEOUT_SECS: u64 = 5;
pub const PING_INTERVAL_SECS: u64 = 2;
pub const DEFAULT_INITIAL_PRICE: f64 = 100.0;
```

### Price Generation Logic:
- **Random walk**: `new_price = current_price * (1 + random(-0.02, 0.02))`
- **Volume**: Popular tickers (1000-6000), others (100-1100)
- **Initial prices**: Read from configuration file, fallback to default (100.0)
- **Tickers**: Read from file at server startup (one per line)

---

## 6. Workflows

### Server Startup Workflow:

```
1. Load configuration from TOML file (server_config.toml)
   - TCP address, tickers file path, quote rate, keepalive timeout, initial prices
2. Read tickers from file → Vec<String>
3. Initialize price state for all tickers (HashMap<String, f64>)
4. Create MPMC channel for quotes
5. Spawn generator thread
6. Start TCP listener on main thread
7. Accept client connections in loop
```

### Client Connection Workflow:

```
1. Client connects via TCP
2. Client sends: STREAM udp://127.0.0.1:34254 AAPL,TSLA
3. Server parses command
   ├─> Valid: Respond "OK", spawn client thread
   └─> Invalid: Respond "ERR <message>", close connection
4. Client thread:
   ├─> Subscribe to MPMC channel
   ├─> Filter quotes by client's tickers (silently ignore unknown tickers)
   ├─> Send filtered quotes via UDP
   └─> Monitor UDP for PING messages
5. If no PING for 5 seconds → terminate thread
```

### Client Workflow:

```
1. Parse CLI arguments (--server-addr, --udp-port, --tickers-file)
2. Read tickers from file
3. Bind UDP socket to local port
4. Connect to server via TCP
5. Send STREAM command with UDP address and tickers
6. Wait for OK/ERR response
   ├─> ERR: Log error and exit
   └─> OK: Continue
7. Spawn UDP receiver thread (listen for quotes)
8. Spawn ping thread (send PING every 2s)
9. Main thread waits for Ctrl+C
10. On interrupt: cleanup and exit
```

### Quote Generation Workflow:

```
Generator Thread (infinite loop):
1. For each ticker:
   ├─> Apply random walk to price
   ├─> Generate random volume
   ├─> Get current timestamp
   └─> Create StockQuote
2. Send all quotes to MPMC channel
3. Sleep for configured duration (default: 1s)
4. Repeat
```

**Invalid Ticker Handling:** Server silently ignores tickers not in its list (simplest approach)

---

## 7. Configuration Approach

### Server Configuration:

**File: `server_config.toml` (hardcoded filename in current directory)**

```toml
tcp_addr = "127.0.0.1:8080"
tickers_file = "tickers.txt"
quote_rate_ms = 1000
keepalive_timeout_secs = 5

# Initial prices for tickers
[initial_prices]
AAPL = 150.0
GOOGL = 140.0
TSLA = 250.0
MSFT = 380.0
NVDA = 500.0
AMZN = 180.0
META = 350.0
JPM = 155.0
# Add more as needed
```

**Loading Logic:**
- Read from `server_config.toml` in current directory
- If file not found: error and exit (config is required)
- If ticker in tickers.txt has no initial price: use default (100.0)
- Log loaded configuration on startup

**Dependencies:** Add `toml = "0.8"` to server's `Cargo.toml`

### Client Configuration:

**CLI Arguments only** (using `clap`):

```bash
quote_client \
  --server-addr 127.0.0.1:8080 \
  --udp-port 34254 \
  --tickers-file tickers.txt
```

**No config file needed** - all settings via CLI for simplicity

### Tickers File Format:

**Format (both server and client):**
```
AAPL
GOOGL
TSLA
MSFT
```

**Parsing Rules:**
- One ticker per line
- Trim whitespace
- Skip empty lines
- Case-sensitive
- No comments or special syntax

---

## 8. Logging Approach

### Logging Stack:

**Dependencies:**
- `log` - Logging facade (macros: `info!`, `warn!`, `error!`, `debug!`)
- `env_logger` - Simple logger with timestamps

### Log Levels:

**ERROR:** Critical issues (with location and backtrace)
- Failed to load config
- Network errors that crash threads
- Unable to bind sockets
- Include: file location, line number, error context, backtrace

**WARN:** Recoverable issues
- Client timeout (no PING received)
- Invalid STREAM command received
- UDP packet send failures

**INFO:** Important events (default level)
- Server started
- Client connected
- Streaming started/stopped
- Configuration loaded
- Quotes received (client side)

**DEBUG:** Detailed diagnostics
- Quote generated
- PING received
- Individual packet details

### Usage Examples:

**Server:**
```rust
info!("Server started on {}", config.tcp_addr);
info!("Client connected from {}", addr);
warn!("Client {} timed out (no PING)", client_id);

// Use log_error! for errors with location and backtrace
if let Err(e) = send_udp_packet() {
    log_error!(e, "Failed to send UDP packet");
}

debug!("Generated quote: {:?}", quote);
```

**Client:**
```rust
info!("Connecting to server at {}", server_addr);
info!("Received quote: [{}] ${}", quote.ticker, quote.price);

// Use log_error! for errors with location and backtrace
if let Err(e) = send_ping() {
    log_error!(e, "Failed to send PING");
}

debug!("UDP packet received: {} bytes", size);
```

### Configuration:

**Runtime control via environment variable:**
```bash
# Default: info level
cargo run

# Show all logs (including debug)
RUST_LOG=debug cargo run

# Show only errors
RUST_LOG=error cargo run
```

**Format:** `env_logger` default format with timestamps:
```
[2025-11-10T14:23:45Z INFO  quote_server] Server started on 127.0.0.1:8080
[2025-11-10T14:23:46Z INFO  quote_server] Client connected from 127.0.0.1:54321
```

**Error Format (with location and backtrace):**
```
[2025-11-10T14:23:47Z ERROR quote_server] Failed to load configuration
  at src/config.rs:42:9
  Error: No such file or directory (os error 2)
  Stack trace:
    0: quote_server::config::load_config
       at src/config.rs:42
    1: quote_server::main
       at src/main.rs:15
    ...
```

---

## Summary

This vision document defines a **minimal, testable implementation** of a real-time quote streaming system:

- ✅ **Simple technology stack** - Standard Rust + 8 essential crates
- ✅ **Clear architecture** - Thread-per-client, MPMC broadcasting
- ✅ **Minimal data model** - One core struct, simple configuration
- ✅ **Straightforward workflows** - Linear, easy to implement
- ✅ **File-based configuration** - TOML for server, CLI for client
- ✅ **Simple logging** - Text output with timestamps

**Next Steps:**
1. Create workspace structure
2. Implement `quote_common` library
3. Build `quote_server` application
4. Build `quote_client` application
5. Test and iterate

**Remember:** Keep it simple, test thoroughly, refactor only when needed.

