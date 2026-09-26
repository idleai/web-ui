# UI foundations

Shared Dioxus components for Idle's browser and VS Code clients. Components render
`app-core` state and emit callbacks; hosts own credentials, networking and native
actions. See the [README](../README.md) for setup, checks and preview commands.

## Themes

Load [theme.css](../crates/web-ui/assets/theme.css) once from the host, or bundle
`theme::STYLESHEET`. Wrap the UI in
[`ThemeProvider`](../crates/web-ui/src/theme.rs) and choose:

- `Theme::{System, Light, Dark, VsCode}`
- `Density::{Comfortable, Compact}`

Use scoped `--idle-*` color, typography, spacing and geometry tokens.
`VsCode` maps injected `--vscode-*` variables onto these tokens with browser
fallbacks. Override tokens on the theme root in a stylesheet loaded afterward.

## Components

| Module | Components |
| --- | --- |
| [controls](../crates/web-ui/src/controls.rs) | Button, IconButton, TextField, Select, Checkbox, Disclosure |
| [icons](../crates/web-ui/src/icons.rs) | Icon, IconName — local SVGs |
| [status](../crates/web-ui/src/status.rs) | StatusBadge, LoadingState, ErrorState, EmptyState |

Callers supply values, callbacks, validation, expansion and pending/error state.
Use meaningful labels and stable, document-unique IDs. The
[gallery](../crates/web-ui/examples/foundations.rs) shows usage in browser and
simulated VS Code hosts with in-memory actions.

Native keyboard behavior, focus rings, reduced motion and high contrast are built
in. Disabled controls leave tab order; busy buttons retain focus and block repeat
actions. Text-field Enter/Escape callbacks preserve modified keys and IME input.
Hosts must restore focus when externally collapsing a section containing it.

## Host capabilities

Both host kinds start with empty
[`HostCapabilities`](../crates/web-ui/src/host.rs). Enable capabilities only after
installing adapters for external links, clipboard, file opening, diffs or reveal.
`HostActionButton` disables unsupported actions with a visible explanation.

Handle `HostRequest` in the user gesture, recheck authorization and validate
targets in the adapter. Report completion through caller state; dispatch alone
does not imply success. `HostError` distinguishes unavailable, cancelled, denied
and failed outcomes.

File targets identify a repository, relative path and revision. `revision: None`
requests the working copy; missing historical content must never fall back to
current content. Position columns are one-based Unicode scalars; adapters convert
them to native editor coordinates.

Legacy EditChain `main.css` remains until its consumers migrate to this theme.
