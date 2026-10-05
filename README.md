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

Cargo resolves the app-core, host-tools and engine packages from their GitHub
releases using `Cargo.lock`. CI checks out this repository. The Rust toolchain
is pinned in `rust-toolchain.toml`.
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

## Package releases

Our reusable crates are stored as `.crate` assets in this repository's GitHub
Releases. The `cargo-index` branch contains the Cargo sparse index; its entries
include immutable archive checksums. `.cargo/config.toml` registers the indexes.
Normal checks need only this repository's source. They refresh internal Cargo
versions before building; the committed lockfile supplies the initial third-party
selection rather than holding internal packages to an older release.

A successful `main` CI run starts the Release workflow. Release-plz calculates
versions and changelogs, and automation commits that metadata to `main`. The
entire CI workflow checks the version commit before any package is published.
Package archives, indexes and native bundles then publish from that exact commit;
there is no separate release PR. Concurrent changes to `main` are never overwritten.

Declare breaking changes in the feature PR, including the required minimum
versions in consumers. Release-plz uses commit messages and Rust API checks to
calculate the next version. To recover a failed publication, use **Re-run failed
jobs** on that Release run, retaining its verified commit even if `main` has
advanced. Dispatch **Release** on `main` to prepare current changes or resume a
current version commit. Existing versions and public archives remain immutable;
retries can complete unfinished drafts. A documentation-only change that does not alter packaged
contents does not create another package version.

Every PR and main CI run resolves the latest compatible internal Cargo packages
and complete native/consumer releases before checking the code. Native and
consumer manifests declare Cargo-style version ranges, such as `^0.1.2`, instead
of fixed release tags and archive checksums. The resolver verifies published
checksums and records the selected versions in ignored
`target/released-dependencies.json`. All jobs in that CI run use this selection;
release verification, publication and native platform builds reuse it as well.

A new build of the same source commit can select newer dependencies. CI retains
its dependency record as an artifact, and releases include that record alongside
their packages. Release preparation incorporates the selected Cargo dependencies
in the version commit, so dependency changes can produce new binaries without a
separate dependency PR. Existing published package versions remain immutable.

The **Check latest released dependencies** workflow compares releases every 15
minutes, or on manual request, and starts ordinary main CI when its inputs have
changed. It creates no branch or PR. PR builds resolve immediately and do not
wait for that schedule. Failed selections remain visible in CI; rerun CI to retry
the same selection, or publish a fix to trigger a new check. Dependabot version
updates remain paused and do not participate in this internal dependency flow.

Keep consumer version requirements accurate when code starts using a new API.
A requirement of `^0.1.2` accepts `0.1.3`; adopting `0.2.0` requires an explicit
requirement change. Canonical `scripts/lint.sh` and `scripts/check.sh` also resolve
latest dependencies. For an individual local command, use:

```sh
python3 scripts/release_dependencies.py run -- cargo build --workspace --locked
```

Resolution uses the authenticated GitHub CLI (`gh`) to discover published assets.
Within one build, `--locked` keeps later commands on the selection that was just
resolved; it does not prevent the next build from selecting newer releases.

Dependabot requires a secret reference for custom Cargo registries, including
public ones. Set the repository's Dependabot secret `PUBLIC_CARGO_REGISTRY_TOKEN`
to the literal value `anonymous`. This is a public marker, not an access token;
the GitHub indexes remain anonymously readable.


## Coordinated development

For ordinary local Rust work, add a temporary Cargo patch for the relevant
registry and pass it with `cargo --config /absolute/path/local.toml ...`.
Keep these overrides out of committed manifests and lockfiles. Full checks with
an unpublished producer can use `memos/scripts/check-integration.py` with
explicit `--producer` and `--consumer` checkout paths. It temporarily patches
Cargo, builds candidate native bundles when needed, runs the consumer's normal
check script and restores its dependency files. The manual **Unpublished package
integration** workflow in memos runs the same check for selected branches.
