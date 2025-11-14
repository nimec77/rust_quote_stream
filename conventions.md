# Code Development Conventions

> **Context:** Read `vision.md` for full technical design, architecture, and project structure.

This document contains **actionable coding rules** for implementation. Follow these conventions strictly.

---

## 1. General Principles

- **KISS First:** Choose the simplest solution that works
- **No Premature Optimization:** Profile before optimizing
- **Explicit Over Implicit:** Make intentions clear in code
- **Single Responsibility:** One function/module = one purpose
- **Fail Fast:** Return errors immediately, don't mask them

---

## 2. Error Handling

### Rules:
- ✅ Use `Result<T, QuoteError>` for all fallible operations
- ✅ Use `?` operator to propagate errors
- ✅ Convert external errors to `QuoteError` variants
- ❌ **NEVER** use `.unwrap()` or `.expect()` in production code paths
- ✅ Use `.unwrap()` only in tests or after explicit checks
- ✅ Provide context in error messages
- ✅ **Capture location and backtrace** when creating errors
- ✅ Use error creation macros that automatically capture `file!()`, `line!()`, `column!()`
- ✅ Include backtrace information in error logs for debugging

### Error Location Tracking:

Errors should capture where they occurred for easier debugging. Use helper macros:

```rust
// Error creation with location
let err = quote_error!(IoError(io_err), "Failed to read config");

// Error logging with location (custom macro)
log_error!(err, "Configuration loading failed");

// This produces output like:
// [ERROR] Configuration loading failed
//   Location: src/config.rs:42:5
//   Error: Failed to read config: No such file or directory
//   Backtrace:
//     0: quote_server::config::load_config
//     1: quote_server::main
//     ...
```

### Example:
```rust
// Good - with location tracking
fn read_tickers(path: &str) -> Result<Vec<String>, QuoteError> {
    let content = fs::read_to_string(path)
        .map_err(|e| quote_error!(IoError(e), "Failed to read tickers file: {}", path))?;
    Ok(content.lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

// When logging errors, use log_error! macro
match read_tickers("tickers.txt") {
    Ok(tickers) => info!("Loaded {} tickers", tickers.len()),
    Err(e) => {
        log_error!(e, "Failed to load tickers");
        return Err(e);
    }
}

// Bad - unwrap in production
fn read_tickers(path: &str) -> Vec<String> {
    fs::read_to_string(path).unwrap()  // ❌ Will panic
        .lines().collect()
}
```

### Backtrace Usage:

- Enable with `RUST_BACKTRACE=1` environment variable
- Backtraces are automatically captured in `QuoteError`
- Use `RUST_BACKTRACE=full` for complete trace with all frames
- In production logs, include abbreviated backtrace (top 5-10 frames)

---

## 3. Naming Conventions

### Variables & Functions:
- `snake_case` for variables, functions, modules
- Descriptive names: `client_thread`, not `ct`
- Boolean prefix: `is_`, `has_`, `should_`

### Types:
- `PascalCase` for structs, enums, traits
- Descriptive: `StockQuote`, `QuoteError`

### Constants:
- `SCREAMING_SNAKE_CASE` for constants
- Group related constants together

### Files:
- `snake_case.rs` for module files
- Match module name to filename

---

## 4. Code Organization

### Module Structure:
```rust
// At top of each file:
use std::...;           // Standard library
use external_crate::*;  // External crates
use crate::*;           // Internal crates

// Order: imports, constants, types, functions, tests

// Constants
const MAX_BUFFER: usize = 1024;

// Types
pub struct MyStruct { ... }

// Functions
pub fn my_function() { ... }

// Tests
#[cfg(test)]
mod tests { ... }
```

### Function Size:
- Keep functions under 50 lines when possible
- Extract complex logic into helper functions
- One level of abstraction per function

---

## 5. Threading & Concurrency

### Rules:
- Use `Arc<T>` for shared immutable data
- Use `Arc<Mutex<T>>` for shared mutable state (minimize usage)
- Use `crossbeam::channel` for MPMC communication
- Always `.join()` spawned threads for cleanup
- Name threads for debugging: `.spawn(|| { ... }).name("worker".to_string())`

### Example:
```rust
// Good - proper thread cleanup
let handle = thread::spawn(|| {
    // work
});
handle.join().expect("Thread panicked");

// Bad - orphaned thread
thread::spawn(|| {
    // work - no join, thread orphaned
});
```

---

## 6. Logging

### Rules:
- Initialize logger once in `main()`: `env_logger::init();`
- Use appropriate levels:
  - `error!()` - Critical failures (use `log_error!()` macro for errors with backtraces)
  - `warn!()` - Recoverable issues
  - `info!()` - Important events
  - `debug!()` - Detailed diagnostics
- Include context in log messages
- **Include location information** in error logs (file, line, function)
- **Include backtrace** for critical errors
- ❌ **NO `println!()` or `print!()`** - use logging only

### Example:
```rust
info!("Server started on {}", addr);
warn!("Client timeout: {}", client_id);

// For errors, use log_error! macro which includes location and backtrace
if let Err(e) = bind_socket() {
    log_error!(e, "Failed to bind socket");
}

debug!("Generated quote: {:?}", quote);
```

### Error Logging Format:

When logging errors, include:
1. Error message
2. File location (file:line:column)
3. Context/operation that failed
4. Stack trace (top frames)

```rust
// log_error! macro produces:
// [ERROR] Failed to bind socket
//   at src/main.rs:42:9
//   in function: quote_server::main
//   Error: Address already in use (os error 48)
//   Stack trace:
//     0: std::net::TcpListener::bind
//     1: quote_server::main
//     ...
```

---

## 7. Testing

### Rules:
- Write tests inline: `#[cfg(test)] mod tests { ... }`
- Test all public functions with complex logic
- Test naming: `test_<what>_<scenario>_<expected>`
- Use `assert_eq!()`, `assert!()`, `assert!(matches!(...))`
- Test error cases, not just happy paths

### Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stream_command_valid_input() {
        let cmd = "STREAM udp://127.0.0.1:8080 AAPL,TSLA";
        let result = parse_stream_command(cmd);
        assert!(result.is_ok());
        let req = result.unwrap();
        assert_eq!(req.udp_addr, "127.0.0.1:8080");
        assert_eq!(req.tickers, vec!["AAPL", "TSLA"]);
    }

    #[test]
    fn test_parse_stream_command_invalid_format() {
        let cmd = "INVALID COMMAND";
        let result = parse_stream_command(cmd);
        assert!(result.is_err());
    }
}
```

---

## 8. Configuration & Constants

### Rules:
- No magic numbers in code - define constants
- Group constants by purpose
- Use configuration structs for related settings
- Validate configuration on load

### Example:
```rust
// Good
const DEFAULT_TIMEOUT_SECS: u64 = 5;
const MAX_RETRIES: u32 = 3;

if elapsed > DEFAULT_TIMEOUT_SECS { ... }

// Bad
if elapsed > 5 { ... }  // ❌ Magic number
```

---

## 9. Serialization

### Rules:
- Use `serde_json` for all serialization/deserialization
- Handle serialization errors gracefully
- Use `#[derive(Serialize, Deserialize)]` on data structures
- Don't panic on malformed JSON - return `QuoteError`

### Example:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockQuote {
    pub ticker: String,
    pub price: f64,
    pub volume: u32,
    pub timestamp: i64,
}

// Serialization
let json = serde_json::to_string(&quote)
    .map_err(|e| QuoteError::SerializationError(e.to_string()))?;

// Deserialization
let quote: StockQuote = serde_json::from_str(&data)
    .map_err(|e| QuoteError::SerializationError(e.to_string()))?;
```

---

## 10. Network Programming

### TCP:
- Set timeouts on sockets where appropriate
- Handle connection errors gracefully
- Close connections explicitly
- Read/write in loops with proper error handling

### UDP:
- Handle packet loss silently (no errors)
- Set buffer sizes appropriately (1024 bytes sufficient)
- Don't assume packet delivery or ordering
- Handle `WouldBlock` errors for non-blocking sockets

### Example:
```rust
// Good - handle partial reads
let mut buffer = [0u8; 1024];
match socket.recv_from(&mut buffer) {
    Ok((size, addr)) => {
        let data = &buffer[..size];
        // process data
    }
    Err(e) => {
        warn!("UDP receive error: {}", e);
        // continue, don't crash
    }
}
```

---

## 11. Comments & Documentation

### Rules:
- Comment **why**, not **what** (code shows what)
- Use `///` doc comments for public APIs
- Comment complex algorithms or non-obvious logic
- No commented-out code in commits
- Keep comments up-to-date with code

### Example:
```rust
// Good - explains why
// Random walk ensures realistic price volatility within ±2% range
let change = rand::thread_rng().gen_range(-0.02..0.02);

// Bad - states obvious
// Generate random number
let change = rand::thread_rng().gen_range(-0.02..0.02);
```

---

## 12. Code Quality Checklist

Before committing code, verify:

- [ ] Compiles without errors: `cargo build`
- [ ] Passes clippy: `cargo clippy -- -D warnings`
- [ ] Formatted: `cargo fmt`
- [ ] Tests pass: `cargo test`
- [ ] No `unwrap()` in production paths
- [ ] Proper error handling with `Result<T, E>`
- [ ] Errors created with location tracking (`quote_error!` macro)
- [ ] Error logs include location and backtrace (`log_error!` macro)
- [ ] Logging instead of `println!()`
- [ ] Constants for magic numbers
- [ ] Threads properly joined
- [ ] Resources cleaned up (sockets, files)
- [ ] Functions tested with unit tests

---

## 13. Specific Project Rules

### StockQuote Structure:
- Always use `chrono::Utc::now().timestamp_millis()` for timestamps
- Prices must be `f64` with 2 decimal places in output
- Volume must be `u32`
- Tickers must be uppercase strings

### Protocol Adherence:
- TCP command format: `STREAM udp://<ip>:<port> <ticker1>,<ticker2>,...`
- TCP response: `OK` or `ERR <message>` (single line, no newline variations)
- UDP PING: Plain text `"PING"` (no JSON)
- UDP quotes: JSON-serialized `StockQuote`

### File Handling:
- Trim whitespace from all file inputs
- Skip empty lines
- Handle missing files with clear error messages
- Use `QuoteError::IoError` for file errors

---

## 14. Anti-Patterns to Avoid

❌ **Don't:**
- Use `.unwrap()` without explicit safety checks
- Use `println!()` for any output (use logging)
- Create threads without joining them
- Ignore return values from functions
- Use `panic!()` in library code
- Write functions longer than 100 lines
- Use generic variable names (`x`, `tmp`, `data`)
- Copy-paste code (extract to function instead)
- Commit code with compiler warnings
- Leave TODO comments without tracking them

---

## Quick Reference

```rust
// Error handling with location tracking
fn my_func() -> Result<T, QuoteError> {
    something().map_err(|e| quote_error!(IoError(e), "Operation failed"))?;
    Ok(result)
}

// Error logging with location and backtrace
match risky_operation() {
    Ok(val) => info!("Success: {}", val),
    Err(e) => log_error!(e, "Operation failed"),
}

// Regular logging
env_logger::init();  // in main()
info!("Message: {}", value);
warn!("Warning: {}", issue);

// Threading
let handle = thread::spawn(|| { ... });
handle.join().unwrap();

// Channel
let (tx, rx) = crossbeam::channel::unbounded();

// Serialization
serde_json::to_string(&data)?;
serde_json::from_str::<T>(&json)?;

// Testing
#[cfg(test)]
mod tests {
    #[test]
    fn test_name() { ... }
}
```

---

**Remember:** When in doubt, choose simplicity. Refer to `vision.md` for architecture and design decisions.

