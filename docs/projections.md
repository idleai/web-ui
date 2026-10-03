# Projection components

`web_ui::projections` renders app-core's activity, task, error, triage and human
input views. The host chooses layout and density and dispatches typed projection
events into its existing app-core instance. Provider reads, authorization,
controller meanings and history retrieval remain in their existing adapters.

Load `theme::STYLESHEET` and `projections::STYLESHEET` once per document (or bundle
`assets/theme.css` and `assets/projections.css`). The styles use the shared theme
tokens, including VS Code palettes, contrast, keyboard focus and reduced motion.

```rust
use app_core::{Event, ViewModel};
use app_core::projections::ProjectionKind;
use dioxus::prelude::*;
use web_ui::projections::{ProjectionLayout, ProjectionPanel};
use web_ui::theme::{Density, Theme, ThemeProvider};

#[component]
fn TaskSidebar(view: ViewModel, onaction: EventHandler<Event>) -> Element {
    rsx! {
        ThemeProvider { theme: Theme::VsCode, density: Density::Compact,
            ProjectionPanel {
                id: "workspace-tasks",
                view: view.projections,
                kind: ProjectionKind::Task,
                layout: ProjectionLayout::List,
                onaction: move |event| onaction.call(Event::Projections(event)),
            }
        }
    }
}
```

The host connects `Event::Projections(Connect(context))` using the authorized
workspace/provider/contributor/chain binding, resolves each `Effect::Projection`
through its original continuation and supplies the resulting `view.projections`.
Use the same dispatcher for every composition. Panels never load or change
semantic state on mount. Production providers that are not connected should
supply unavailable results through the existing app-core contract.

## Composition

| Component | Contents |
| --- | --- |
| `ProjectionPanel` | Destination heading, read feedback, summary, filters, selection feedback, chosen layout and limitations. Requires a document-unique `id`. |
| `ProjectionList` | Compact rows in provider order. |
| `ProjectionTable` | Native column/row headers and a labelled, keyboard-focusable horizontal scroll region. |
| `ProjectionCards` | Cards wrapping to the available container width. |
| `ProjectionCard` | One row with selection, status, labels, summary and record actions. |
| `ProjectionBoard` | Columns grouped by exact status, in first-seen order; row order remains unchanged within each column. Missing and empty statuses stay separate. |
| `ProjectionSummary` | Supplied total, loaded count, visible count, freshness, coverage and optional generation/checkpoint details. |
| `ProjectionFilters` | Controlled literal text, exact status and conjunctive label filters. Requires a document-unique `id`. |
| `ProjectionFeedback` | Loading, read/action errors, suspended connection and explicit refresh. |

Use `ProjectionPanel` for a complete destination. For custom compositions, pair
the individual layouts with summary, feedback and the supplied gaps. Layouts take
a `ProjectionView`, the shared `Option<ProjectionSelection>` and the same
`EventHandler<projections::Event>` as the panel; `ProjectionCard` takes its row
and kind directly. They do not duplicate counts or a toolbar in every row.

Compact density reduces spacing and control height; coarse pointers retain larger
targets. Cards and boards wrap/stack within their container, even in a sidebar on
a wide page. Tables keep all columns inside a local scroll region. Hosts choose
the layout; no viewport detection, routing or application state is hidden in the
components. Browser and extension mounts can coexist with distinct panel IDs.

## Counts, freshness and limitations

Scope total, loaded count and visible count are rendered from the supplied view,
including the full `u64` range. A missing total is shown as **Total unknown**.
Filtering never changes the supplied total or loaded count. Boards do not infer
lane totals or workflow transitions; they group display rows only.

Freshness is independent of coverage: a partial result can be current for its
declared coverage. A generation timestamp or opaque checkpoint never establishes
currentness. Optional result details retain both values exactly. Known empty,
filtered empty, partial, unavailable, initial loading and disconnected states have
distinct messages. Failed refreshes and interrupted connections retain app-core's
stale rows. Refresh is disabled during reads and suspension; hosts own reconnect.

Supplied gap messages and full addresses remain visible alongside results.
Standalone gap addresses have no inspection button because app-core's `Inspect`
requires a reference supplied on a row. They are never assigned to a guessed row.

## Selection, filters and records

Selection emits `Select(Some(ProjectionSelection { kind, key }))` using the exact
provider key. Native buttons support Enter and Space and report `aria-pressed`.
Clear selection emits `Select(None)`. App-core retains a selected key when filters
hide it, and the panel reports that condition. Replacement rows use stable keys;
inserting rows preserves retained DOM, keyboard focus and open record details.
Retractions and context changes follow app-core's returned selection.

`SetFilter` carries the complete filter. Text is a literal, case-sensitive
substring in the title or summary. Status uses exact equality. Every selected
label must match. Status and label choices come from shown rows plus active
filters; **Clear filters** restores all loaded choices. Active choices remain
removable even with no matching rows. Commas, whitespace and empty labels are
preserved as individual opaque values; an empty status is distinct from no filter.

Each row's **Records** disclosure exposes source and related addresses separately.
Inspection emits `Inspect { selection, reference }` with the complete supplied
observation, logical item and record digest. Item-only and observation-only links
remain distinct. No prefixes, URLs or inferred addresses replace those fields.
App-core validates the link and routes it into history; hosts mount the shared
history details surface and execute its effects. Record inspection does not also
select the enclosing projection row. Provider strings render as text, preserving
line breaks and escaping markup.

## Preview and checks

```sh
./scripts/build-projections.sh
python3 -m http.server 4173 --bind 127.0.0.1 --directory out/projections
CHROME_PATH=/path/to/chrome npm run test:projections:browser
./scripts/lint.sh
./scripts/check.sh
```

The development preview uses a real app-core instance and explicit presentation
scenarios behind its projection effect boundary. It shows light browser and dark
compact sidebar compositions sharing filters, selection and refresh state. Use
the fixture controls to complete/fail a pending read, interrupt/reconnect, insert
or retract rows, or supply empty/partial/unavailable results. The preview supplies
no live controller or history-content adapter; record selection still runs through
app-core. These scenarios live only in the example and tests.

Native tests cover counts, freshness, empty states, semantic markup and exact
typed actions. Browser tests cover all four layouts, both compositions, keyboard
selection, filters, record inspection, keyed updates, reconnect and 320px layouts.
They save wide/narrow screenshots under `out/test-artifacts/`. CI builds the
preview and runs the same browser tests after the canonical Rust checks.

The host assembly features f43/f60 own production mounts and provider bindings;
f11/f13 and f54 retain live controller and managed projection integration.
