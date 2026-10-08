# History rows and details

`web_ui::history::details` renders app-core's history views in browser and
extension hosts. The three public components are:

- `HistoryRow`: controlled item disclosure, bounded previews, all loaded message
  blocks, tool attempts/channels and observation metadata.
- `HistoryDetails`: selected observations, file snapshots/comparisons, exact
  stored encodings and linked Original content. It uses `selected_item` even
  when that item is outside the filtered or mounted graph.
- `HistoryTimeline`: the existing `HistoryGraph` with `HistoryRow` in its content
  slot and a persistent `HistoryDetails` inspector below it.

The full Activity editor uses [HistoryExplorer](history-explorer.md), a continuous
table over native timeline windows. Its rows open normal VS Code file, diff or
operation JSON editors. The components described here serve existing raw-history
and recorded-session callers.

The graph continues to own geometry, scrolling, focus and row measurement. The
components do not reconstruct streams, resolve files or maintain another copy of
selection, disclosure or request state.

## Integration

Bundle the foundation, graph and details stylesheets once per document:

```rust
web_ui::theme::STYLESHEET;
web_ui::history::graph::STYLESHEET;
web_ui::history::details::STYLESHEET;
```

Mount `HistoryTimeline` with a document-unique `id`, the current
`app_core::history::ViewModel`, an explicit `HostCapabilities` snapshot and an
`onaction: EventHandler<app_core::history::Event>`. Dispatch each event to the
owning app-core runtime and pass its next view back to the component. Hosts can
also supply `HistoryRow` through `HistoryGraph::render_item` and place
`HistoryDetails` elsewhere in their layout. Each row's disclosure ID must be
unique across all mounts.

The optional `activity` property on both detail components accepts supplied
author classifications and mapped code ranges. The shared author/activity panel
also presents known file-level actions when that input is absent. See the
[author activity guide](author-activity.md) for revision matching, unknown
coverage and the provider boundary.

| User action | Typed event |
| --- | --- |
| Expand or collapse an item | `ToggleDisclosure(full_item_key)` |
| Inspect an item or observation | `Select(Selected { item, observation })` |
| Continue or retry the item scan | `LoadItem(full_item_key)` |
| Load, retry or refresh exact fields/Originals | `LoadOperationDetails { operation, refresh }` |
| Open a stored record, Original, revision or comparison | `Open { record: RecordRef, target: OpenTarget }` |

An item scan can finish while a block or field remains unavailable. The UI labels
these states separately. Request errors stay visible, with retry controls; late
blobs become visible after a successful refresh through app-core.

## Native adapters

`HostCapabilities::supports_history` maps native history targets to independently
advertised adapters:

| `OpenTarget` | `HostCapability` |
| --- | --- |
| `Record` | `OpenRecord` |
| `Original` | `OpenOriginal` |
| `File` | `OpenFile` |
| `Diff` | `OpenDiff` |

Both host kinds start with all adapters unavailable. Unavailable actions remain
visible, disabled and associated with a reason. Changing the capability snapshot
revokes the action on the next render, including synthetic activation. Pending
opens cannot dispatch again. Success is never inferred from a click; pending and
error feedback comes from `ViewModel::open`.

History opens use app-core events/effects, rather than converting a recorded path
identity into a `FileTarget`. The host executes `QueryAction::Open` in the selected
chain, retaining **both** the full operation ID and the stored-record digest. It
must resolve repository bindings, recheck access and report missing/unavailable
content through the existing effect result. A historical request must never be
replaced with a working-copy open. f40 owns extension-native resolution; f43/f60
connect the production extension/browser hosts.

## Content fidelity

- Row previews stop at 320 bytes without splitting UTF-8. They show the displayed
  byte count, total loaded byte count and an explicit truncation notice. Hidden
  block counts are visible. An incomplete app-core `ContentText` stays labelled
  incomplete even when the item is expanded.
- Expansion shows every cached block and observation. Message blocks use their
  recorded positions when supplied. Tool attempt and channel identities stay
  attached to each block, including reused block IDs. Missing prefixes and
  recorded terminal lifecycle are independent labels.
- Full fields are loaded by observation. Empty, absent, missing, corrupt and
  unresolvable content have different labels. Conflicted operations show every
  retained encoding and its digest without selecting one as accepted content.
- File observations preserve path **identity**, revision, proposed/applied
  state, before/after content identities and causing call. Exact snapshot fields
  are shown alongside the engine's byte comparison. An edit alone does not imply
  that a complete snapshot exists.
- Original drill-down loads the linked observation separately. Stored encodings
  come from `RawRecord::bytes`; captured Original content comes from its exact
  fields. `operation_json` and row previews never replace those bytes.
- Content is rendered as escaped text, without Markdown/HTML execution or JSON
  reformatting. Binary data uses hexadecimal; exact text fields also include
  hexadecimal so CRLF, NUL, whitespace and other control bytes remain inspectable.
  Scrollable, keyboard-focusable regions contain all loaded bytes, with no hidden
  character or line limit.

Author, recorder, observation and logical item remain separate identities. All
causal parents, logical causes, converter and legacy references remain available
on expanded observations.

## Source ownership

f31 adapts the row/disclosure, content, file and raw-record presentation previously
owned by `editchain-history-renderer/src/app/rows.rs`, `rows/files.rs`,
`rows/markdown.rs` and the associated direct-DOM code. New consumers use Dioxus
components over app-core's full typed data. Legacy coordinate/wire objects,
derived row summaries and stringified native-open envelopes are not inputs to
these components.

Both current hosts use the shared components. The old renderer, its generated
assets and coordinate service have been removed. This repository owns the
details module, host capability variants, stylesheet, example and browser tests;
app-core owns their typed data and reducers. Production provider connections
remain separate f43/f60 work.

## Verification and local preview

Keep app-core, host-tools and EditChain alongside this repository, as described in the
repository README. Run:

```sh
./scripts/check.sh
bash scripts/build-history-details.sh
CHROME_PATH=/path/to/chrome npm run test:history-details:browser
python3 -m http.server 4174 --bind 127.0.0.1 --directory out/history-details
```

The preview uses labelled development-only responses behind the same view/event
boundary. It exercises late blobs, conflicting encodings, two tool attempts,
binary Originals, capability revocation and failed native opens. It does not
connect to a workspace or open native editors.

Rust tests cover byte preservation, UTF-8 bounds, availability states, independent
attempts, attribution, file identities, selected-item retention and typed event
dispatch. Browser tests exercise both host compositions, controlled expansion,
Original drill-down, late snapshot refresh, variant-specific native opens,
capability revocation, focus and narrow layouts. CI runs both the existing graph
suite and the new details suite.

Checked on 2026-10-01 against published app-core `1894a53` and EditChain
`3c75cf0`, in detached dependency checkouts so concurrent app-core work stayed
untouched. `scripts/check.sh` exited 0, including the canonical lint script's
`RESULT: PASS`, 59 Rust tests and native/WASM builds. All seven graph and six
details Chrome scenarios passed without skips. The details suite also checks
that clicking and scrolling a content panel preserves its focus and the graph's
scroll position. Screenshots were inspected in desktop and narrow layouts.
