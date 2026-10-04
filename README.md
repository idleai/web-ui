# web-ui

Shared Dioxus components for Idle's browser and VS Code clients: theme tokens,
accessible controls, icons, status views, virtualized history graphs and exact
record details, shared conversations, prompting, session sharing and projection
lists, tables, cards, task boards and workspace navigation. Components render `app-core` state; hosts own
credentials, networking and native actions.

Source and assets live in [`crates/web-ui/`](crates/web-ui/).

`assembly::WorkspaceSurface` renders the ordered workspace navigation in its
compact sidebar and composes history/details and sessions in wider detail views.
Mount one per document, supply a persistent app-core view and event dispatcher,
and load the theme, navigation, graph, details, session and projection styles. It advertises
only supplied host actions; session creation requires a connected provider and
a prepared request. Detail navigation exposes every workspace destination.

`configuration::ConfigurationEditor` provides independent Settings and Agent Rules
forms with validation, conflicts, saved revisions and recovery controls. Its
`onsave` callback asks the host to allocate and durably retain the write identity.
`resources::ResourceDirectory` renders compute, providers, served models,
installation choices and controller state. It emits typed navigation/recovery
events and runtime mutation intents; capabilities and grants control available
actions. Mount these through `destination` and load `configuration.css` and
`resources.css`. Hosts retain credentials and perform all persistence/execution.

## Development

Keep the `app-core`, `host-tools` and `editchain` checkouts alongside this repository. CI uses
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

See the [author activity guide](docs/author-activity.md) for shared attribution
heatmaps, observed exposure/touch, unknown coverage and exact source drill-down.

See the [session guide](docs/sessions.md) for conversations, attributed prompt
delivery/execution, invitation controls and the browser/extension preview.

See the [projection guide](docs/projections.md) for supplied counts and freshness,
record links, controlled filters and compact/sidebar or wide/browser compositions.

See the [navigation guide](docs/navigation.md) for section ordering, supplied
counts/statuses, identity boundaries, selection events and creation requirements.
