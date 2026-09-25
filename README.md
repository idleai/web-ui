# web-ui

Initial scaffold for Idle. Module ownership, current behavior, and build
instructions are documented below; reserved modules are intentionally empty.

Shared Dioxus 0.7 components render `app-core` view models. Both UI hosts currently
mount one scaffold component. Browser layout belongs here; credentials, transport,
VS Code APIs, semantic state and runtime execution do not.

Keep sibling checkouts under one parent directory:

```text
repos/
  app-core/
  web-ui/
  vscode-extension/
  web/
  editchain/          # existing history engine and extraction source
  codex/              # existing idleai/codex-evo checkout
```

Path dependencies are deliberate during extraction. CI checks out the required
public siblings from `idleai/*` on `main`. The Cargo lockfile pins registry
dependencies, not sibling source revisions; coordinate boundary changes across
repositories. Once contracts are ready for release, replace sibling paths with
versioned packages or pinned Git revisions as a separate packaging change.

Only the `app-core` sibling is required to build this library.

Rust 1.97.0 is selected by `rust-toolchain.toml`. Cargo installs the specified
toolchain/targets on first use; lockfiles are tracked. Install the dependency
policy checker once:

```sh
cargo install cargo-deny --locked --version 0.20.2
```

`./scripts/lint.sh` is the canonical Rust quality gate, shared by local checks and
CI: formatting, locked checks/Clippy/tests with all features, doctests, Rustdoc
and cargo-deny. Browser-facing libraries also run checks and Clippy for
`wasm32-unknown-unknown`. Every package inherits EditChain's strict workspace
Rust, Clippy and Rustdoc rules and its thresholds in `clippy.toml`.
`./scripts/check.sh` runs that gate followed by this repo's builds/packaging.

```sh
bash scripts/check.sh
cargo build --locked --target wasm32-unknown-unknown
```

The hosts copy `assets/theme.css` into their own bundles.

| Boundary | Owner after f1 |
| --- | --- |
| Manifest, exports (including `history.rs`), shared controls, `host.rs`, theme assets | f29/theme-host-api |
| `history/graph.rs` | f30/history-graph |
| `history/details.rs` | f31/history-details |
| `sessions.rs` | f32/session-ui |
| `projections.rs` | f33/projection-ui |
| `navigation.rs` | f34/workspace-navigation |
| `resources.rs` | f35/resource-ui |
| `configuration.rs` | f36/settings-rules-ui |
| `provenance.rs` | f37/provenance-ui |

Reuse presentation from EditChain's history renderer, visual geometry from
`editchain-project/src/layout.rs` and `editchain-protocol/src/live_graph/`, and the
existing extension's `media/main.css` during feature work. f1's small neutral
stylesheet is only enough to mount the shared component; the theme system is f29.

The dependency policy in `deny.toml` includes one explicit maintenance exception:
[RUSTSEC-2025-0141](https://rustsec.org/advisories/RUSTSEC-2025-0141.html), for
Crux 0.20's mandatory `bincode` 1.3.3 dependency. Remove it when Crux migrates
serialization. Other advisories remain checked. Crux's optional macro feature
is disabled, removing its unmaintained `proc-macro-error` dependency.
