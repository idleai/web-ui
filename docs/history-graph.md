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

## Viewport and interaction

Only the visible rows, a small overscan region and an offscreen focused row remain
mounted. Connections crossing the viewport are drawn even when both endpoints are
offscreen. Every graph instance owns its measurements, scroll offset and focus.
An item's identity and pixel offset anchor scrolling through insertion, causal
repair and resizing. If an anchor retracts, the next surviving item retains its
screen position, subject to the scroll bounds. A chain change clears geometry.

The tree uses a stable active descendant. Arrow keys, Home, End and Page Up/Down
move focus and reveal the item; Enter/Space select it; left/right request disclosure
through app-core. Modified keys and IME composition keep native behavior. New
selection targets are revealed when they become available, without taking DOM
focus from another control. Selection and disclosure never become local domain
state.

Vertical positions and anchor corrections apply together without a transition.
New connections grow with the extracted timing planner and new nodes fade in.
Stable SVG keys retain animation instances, and expired entry animations do not
replay when an item returns to the viewport. Reduced-motion preferences disable
both effects. Paths and rows use shared theme tokens.

## Extracted implementation

The `history-geometry` workspace crate owns the former EditChain layout planner,
retained lane/routes/coverage state, connection growth timing and coordinate types.
It has no Dioxus, app-core, host or network dependency. Its generic `GraphNode` and
`GraphRow` adapters let the old viewer use the moved implementation without a
dependency cycle. The new component uses the same lanes and route corners for
causal paths and adds a separate logical-relationship layer.

Lane transitions use cubic curves with vertical tangents at both ends, so forks
and merges meet their tracks without sharp corners. Turns borrow at most half of
each neighboring vertical run, keeping close turns smooth as row heights change.
Logical side routes and self-loops use the same tangent rule; node attachments
stay at their measured centers.

EditChain's layout module, live-graph module, growth module and coordinate module
are compatibility exports/adapters. The legacy direct DOM renderer and its
coordinate-based service remain until f31/f43 replace their content and host
consumers. Remove those adapters with that switch. Engine storage and relationship
queries remain in EditChain.

Land the web-ui geometry package before the paired EditChain consumers. EditChain
CI checks out web-ui beside app-core; the new web-ui package can build against the
previous engine main because its dependencies are only core and index crates.

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
focus, disclosure, removal, narrow layouts, zoom and reduced motion. Screenshots
are written to `out/history-tests/`. The suite also checks SVG derivatives for
sharp joins through disclosure, late arrivals and narrow layouts. CI runs these
checks after the Rust suite.
