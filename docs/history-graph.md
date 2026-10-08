# History graph

`web_ui::history::graph::HistoryGraph` renders app-core's history view and sends
`app_core::history::Event` through its `onaction` callback. The host supplies a
document-unique `id`, dispatches events into Crux and supplies the updated view.
Load `web_ui::theme::STYLESHEET` and `history::graph::STYLESHEET` once in the host.
The browser renderer must enable Dioxus Web's `mounted` feature. No backend,
credentials, VS Code API or semantic reducer lives in the component.

```rust,ignore
rsx! {
    ThemeProvider {
        HistoryGraph {
            id: "workspace-activity",
            view: application_view.history,
            onaction: move |event| dispatch(app_core::Event::History(event)),
        }
    }
}
```

The optional `render_item: Callback<ItemView, Element>` supplies row content.
It receives the complete item, including every cached observation, content block,
disclosure flag and item-scan status. f31 owns the rich content renderer. The
graph measures supplied content with resize events and adjusts its row positions
and scroll anchor. Nested controls should consume their own keyboard actions;
clicks on buttons, links and form controls do not select the surrounding row.

An optional `header` element sits outside the vertical viewport. The graph
publishes `--idle-history-graph-width` so a table header can align with its rows.
Set `disclosure: false` for a mini view that opens details in another surface;
its rows then omit `aria-expanded` and do not consume left/right disclosure keys.
Lane colors can be supplied with `--idle-history-lane-0` through `-5`; the default
remains the host's accent color. See the [activity editor guide](history-explorer.md).

## Relationships and identity

`GraphSnapshot::from_view` retains each observation's full `RecordRef`. It resolves
physical parent IDs through loaded observation membership and includes every
parent, including third and later parents and connections within one item.
Logical causes and each Link destination have separate line styles and types.
Links do not affect causal ordering or lane assignment. Connection keys include
the full observation, record digest, relationship kind and typed endpoint IDs.

Unknown endpoints remain explicit and resolve without changing connection keys.
Conflicting observation-to-item mappings are ambiguous, never resolved by choosing
a hash. Repository-scoped Git endpoints remain external references: the current
F23 commit row does not supply the repository/OID mapping needed to alias them to
a loaded item. A recorded Commit kind can still use the graph's Git column.

Rows are keyed by full logical item ID. Repeated observations update the same DOM
row and retain their separate operation and record-digest references. Item groups
can contain opposing causal connections even when individual observations form a
DAG. The graph reports an ordering problem and draws those connections too.
Rendering clocks are local geometry, never changes to recorded timestamps or
claims about completion.

Lane continuation uses the item's recorded session. If no observation has a
session, a single recorded recorder identity supplies the continuation instead.
Session and recorder IDs remain separate domains; conflicting identities within
an item do not choose an arbitrary source. Appending a linear recorder history
without session metadata keeps the same lane as loading those records together.

## Viewport and interaction

Only the visible rows, a small overscan region and an offscreen focused row remain
mounted. Connections crossing the viewport are drawn even when both endpoints are
offscreen. Every graph instance owns its measurements, scroll offset and focus.
An item's identity and pixel offset anchor scrolling through insertion, causal
repair and resizing. If an anchor retracts, the next surviving item retains its
screen position, subject to the scroll bounds. A chain change clears geometry.

The tree uses a stable active descendant. Arrow keys, Home, End and Page Up/Down
move focus and reveal the item; Enter/Space select it; left/right request disclosure
through app-core. Page Up and Page Down advance even across rows taller than the
viewport. Pointer selection preserves the current scroll offset through the
host's selection update, including clicks partway through a tall row. Modified
keys and IME composition keep native behavior. External selection targets are
revealed when they become available, without taking DOM
focus from another control. Selection and disclosure never become local domain
state.

Vertical positions and anchor corrections apply together without a transition.
New connections grow with the extracted timing planner and new nodes fade in.
Stable SVG keys retain animation instances, and expired entry animations do not
replay when an item returns to the viewport. Reduced-motion preferences disable
both effects. Paths and rows use shared theme tokens.

## Extracted implementation

The `history-geometry` workspace crate owns bootstrap lane planning, retained
routes and coverage, connection growth timing and exact coordinate conversion.
It has no Dioxus, app-core, host or network dependency. Its generic `GraphNode`
and `GraphRow` adapters supply the component's causal lanes and route corners;
the component adds a separate logical-relationship layer.

Lane transitions use cubic curves with vertical tangents at both ends, so forks
and merges meet their tracks without sharp corners. Turns borrow at most half of
each neighboring vertical run, keeping close turns smooth as row heights change.
Logical side routes and self-loops use the same tangent rule; node attachments
stay at their measured centers.

`history-geometry` owns layout, live graph, growth and viewport calculations used
by Dioxus. Its old projection/protocol adapters and the direct DOM renderer have
been removed after both current hosts adopted the shared components. Layout
regressions exercise `LiveGraph` directly, including offscreen connections and
muted paths. Saved row geometry preserves the former snapshot implementation's
results across bootstrap, insertion, retraction and mute edits. EditChain owns storage and queries
and has no dependency on this repository or application display classifications.

## Local checks

`bash scripts/check.sh` runs the canonical Rust lint suite and native/WASM builds.
The history fixture uses only development data behind the production component
interface. Build and test it with:

```sh
npm ci
bash scripts/build-history.sh
CHROME_PATH=/path/to/chrome npm run test:history:browser
```

The browser suite starts its own loopback server, fails if Chrome or built assets
are missing, and checks both host compositions, late records/endpoints, keyboard
focus, tall-row paging and selection, disclosure, removal, narrow layouts, zoom
and reduced motion. Screenshots are written to `out/history-tests/`. The suite
also checks SVG derivatives for
sharp joins through disclosure, late arrivals and narrow layouts. CI runs these
checks after the Rust suite.
