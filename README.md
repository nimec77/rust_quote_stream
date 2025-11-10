# rust_quote_stream

Prototype real-time stock quote streaming system composed of `quote_server`, `quote_client`, and shared `quote_common` crates. Development follows the staged plan outlined in `doc/tasklist.md` and architecture in `vision.md`.

## Workspace Layout

- `quote_common` – shared data types and utilities
- `quote_server` – quote generator and TCP/UDP streaming server
- `quote_client` – CLI client consuming streamed quotes
- `tickers.txt` – default ticker list used by server and client
- `server_config.toml` – baseline server configuration

## Getting Started

```bash
cargo build
```

Further functionality will be implemented iteratively; consult `doc/tasklist.md` for current progress and upcoming work.

