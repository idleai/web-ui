# web-ui

Shared Dioxus components for Idle's browser and VS Code clients: theme tokens,
accessible controls, icons, status views, virtualized history graphs and exact
record details, shared conversations, prompting and session sharing. Components
render `app-core` state; hosts own credentials, networking and native actions.

Source and assets live in [`crates/web-ui/`](crates/web-ui/).

`assembly::WorkspaceSurface` composes the available workspace selector, history
graph/details and session components for compact sidebar or wider detail views.
Mount one per document, supply a persistent app-core view and event dispatcher,
and load the theme, graph, details and session styles. It advertises only supplied
host actions; runtime-backed session actions remain unavailable until the host
connects their provider and draft lifecycle. The full navigation/resource/settings
surfaces retain their roadmap owners.

## Development

Keep the `app-core` and `editchain` checkouts alongside this repository. CI uses
the paired revisions in `.github/workflows/ci.yml`. The Rust toolchain is pinned in `rust-toolchain.toml`.
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

See the [history details guide](docs/history-details.md) for message/tool content,
file revisions, Original drill-down, typed actions and host capabilities.

See the [session guide](docs/sessions.md) for conversations, attributed prompt
delivery/execution, invitation controls and the browser/extension preview.

## Compatibility history renderer

[`editchain-history-renderer`](crates/editchain-history-renderer) renders the
existing history panel. Its host supplies the message API at startup; the VS Code
loader owns platform API acquisition. Its stylesheet is
`crates/editchain-history-renderer/assets/history.css`.

`history-geometry::legacy_protocol` and `legacy_projection` adapt app-core's
shared rows and semantic projections to graph layout. The moved projection/layout
integration tests run in this workspace. The packaged compatibility panel and
its generated assets are built and checked by vscode-extension's
`history-renderer.yml` workflow.
