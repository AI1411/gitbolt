# gitbolt

Git GUI built with Rust and Dioxus Desktop.

## Requirements

- Rust stable (see `rust-toolchain.toml`)
- macOS / Apple Silicon arm64 for MVP development
- [Dioxus CLI (`dx`)](https://dioxuslabs.com/learn/0.7/getting_started) for hot-reload development

## Development

```bash
# Run with hot reload
dx serve --desktop

# Or run directly
cargo run
```

## Checks

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build
```

## Module layout

See `docs/design/05-architecture.md` section 15 for the planned module structure under `src/`.
