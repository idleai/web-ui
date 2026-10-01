# web-ui

Shared Dioxus components for Idle's browser and VS Code clients: theme tokens,
accessible controls, icons, status views and virtualized history graphs. Components render `app-core` state;
hosts own credentials, networking and native actions.

Source and assets live in [`crates/web-ui/`](crates/web-ui/).

## Development

Keep the `app-core` and `editchain` checkouts alongside this repository. CI uses
their `main` branches. The Rust toolchain is pinned in `rust-toolchain.toml`.
Run checks and builds from the repository root:

```sh
cargo install cargo-deny --locked --version 0.20.2
./scripts/check.sh
```

## Preview

```sh
cargo install wasm-bindgen-cli --locked --version 0.2.127
./scripts/build-foundations.sh
python3 -m http.server 4173 --bind 127.0.0.1 --directory out/foundations
```

Open <http://127.0.0.1:4173/> for the interactive component gallery.

See the [development and integration guide](docs/foundations.md) for theme tokens,
component usage, accessibility behavior and host capabilities.

See the [history graph guide](docs/history-graph.md) for app-core integration,
relationship handling, the `history-geometry` package and browser checks.
