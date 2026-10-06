# Workspace navigation

`navigation::WorkspaceNavigation` presents Workspace, Users, Sessions, Projections,
Compute hosts, Model providers and compact Activity in that order, followed by
separate Settings and Agent Rules entries. Control sessions precede runner sessions;
the relative order within each group remains the supplied order. When no Control
session is visible, an ambient control summary opens the Sessions destination
without inventing a session identity.

Section headers collapse and expand with the mouse, Enter or Space. Their toolbar
actions open the section destination without changing expansion. Compact navigation
uses 22-pixel rows, inline member activity and a branch label beside the workspace
picker. Full connection details remain in row tooltips. Repository inspection is
rendered only by `assembly::Surface::Detail`.

Load `theme::STYLESHEET`, `history::graph::STYLESHEET` and
`navigation::STYLESHEET` once in the host, or bundle their corresponding files from
`crates/web-ui/assets/`. Mount under `ThemeProvider`; both comfortable browser and
compact VS Code densities use the same component and events. Supply a unique `id`
for each navigation instance and feed back the persistent app-core root view.
Icons expose their `IconName` through `data-icon`, with a nested SVG fallback.
The VS Code host can provide its Codicon font and workbench styles without
changing the shared Rust renderer or application events.

```rust,ignore
WorkspaceNavigation {
    id: "workspace-sidebar",
    view: core_view,
    onaction: dispatch_to_app_core,
    creation: prepared_creation,
    now_ms: provider_clock,
}
```

`assembly::WorkspaceSurface` mounts this overview for `Surface::Sidebar`. A host
can open its selected destination in a detail surface or its own composition.
The selected history/session details and projection panels remain reachable below
the sidebar; hosts can supply `destination` for their own member, resource or
configuration screens. The sidebar retains its bounded, independently scrollable
Activity graph with 24-pixel preview rows and
uses the existing graph's full record identities, keyboard selection and geometry.
The first supplied record preview labels each compact item; this is a preview,
not a claim that a record is the latest observation.

Hosts with native sidebar views can mount `WorkspaceNavigationPane` for a single
`NavigationSection`, or use `assembly::Surface::SidebarPane(section)`. This mode
renders that section's content. The Activity pane also places the shared Settings
and Agent Rules links below its graph, without additional headers or disclosure
controls. The native host owns the pane's title, toolbar, divider, collapse state
and resizing. The extension uses this pane for Activity alongside six native
trees and routes selections to its detail editor. A host must coordinate workspace binding changes across
its documents and replay the latest selection when a hidden document is recreated.

## State and actions

| Control | App-core intent |
| --- | --- |
| Workspace selector | `Workspace(SelectWorkspace(id))` |
| Repository selector (managed workspaces) | `Workspace(SelectRepository(Some(id)))`; the All repositories choice sends `None` |
| Section heading, Settings, Agent Rules | `Workspace(Navigate(section))` |
| Session row | `Sessions(Select(Some(id)))`, then Sessions navigation |
| Compute row | `Resources(SelectHost(Some(id)))`, then Compute hosts navigation |
| Provider row | `Resources(SelectProvider(Some(id)))`, then Model providers navigation |
| Activity item | Existing `History` selection event, then Activity navigation |
| Prepared Add session | `Sessions(Create { id, draft })`, then Sessions navigation |
| Reload workspaces | `Workspace(Load)` |

Standalone mode shows only the workspace selector. App-core automatically selects
that workspace's single repository; switching workspaces retains the explicit
repository/chain binding. Managed workspaces also expose repository selection.

Users are keyed by contributor ID and display their supplied member names.
Connection summaries, branches and host IDs are secondary, explicitly labelled
context. Multiple connections do not create multiple users. Compute hosts retain
their own resource identities, and selecting them never grants compute access.

Directory counts reflect supplied rows. A not-yet-loaded directory has no count;
retained rows keep their count during refresh or failure. Projection badges use
`total`, including a supplied zero or a large integer; an absent total says
“Total unknown.” Loaded rows, completeness and freshness remain distinct.
Control phase, ownership gaps, effective resource availability, connection state
and peer activity come from app-core. Runner rows show the supplied Owned/Invited
relationship because the current session directory has no session-wide execution
status or age. No relative ages, branch activity or execution states are inferred.

## Creation and current contract limits

A `SessionCreation` contains the exact `SessionContext`, reserved mutation identity
and `SessionDraft`. The host prepares the title and host choice, reserves a fresh
request identity and persists the unchanged payload before executing its effect.
Supply the provider-aligned `now_ms`; missing/elapsed deadlines, mismatched
connections, unavailable capabilities, loading states and used identities disable
creation. A local click guard blocks repeated dispatch before app-core rerenders.
App-core still validates the action and the runtime still authorizes execution.
No session appears until the runtime confirms creation. Recovery uses app-core's
retained mutation and original request identity.

The current app-core contract has no host/provider registration action. Their
plus controls remain disabled with an explanation; resource selection is fully
wired. Registration belongs to the resource/backend owners. There is also no
projection-destination selection or individual-member selection event. Projection
summary rows open the shared Projections section, and user rows or the Users toolbar
open Members. Hosts can compose the existing projection panels there. This component
does not fabricate row selections, registration requests or private routes.

Settings and Agent Rules are independent navigation entries. Their forms remain
owned by the configuration surface; the navigation component performs no writes.

## Verification and preview

```sh
./scripts/lint.sh
./scripts/check.sh
./scripts/build-navigation.sh
CHROME_PATH=/path/to/chrome npm run test:navigation:browser
python3 -m http.server 4173 --bind 127.0.0.1 --directory out/navigation
```

The preview shares a real app-core dispatcher between two presentations. It
extends the existing development session/resource fixtures and supplies isolated
graph presentation data. It performs no runtime or network operations. Native
tests cover exact dispatch, stale/foreign/expired creation requests, duplicate
clicks, supplied totals and workspace changes. Browser tests cover keyboard
selection, both host densities, distinct user identities, expiry/failure states,
unique IDs, narrow layouts and larger text. Screenshots are written under
`out/navigation/screenshots/`.
