# Shared author and observed activity

`web_ui::provenance::AuthorActivity` renders attribution heatmaps, exposure/read
and touch indicators, and exact source drill-down for a selected file record.
`HistoryDetails` mounts it in browser and extension compositions;
`HistoryTimeline` and `WorkspaceSurface` forward their optional `activity` input.
The existing history details stylesheet includes all required styles. f42's
native decorations and commands retain their existing implementation.

## Supplied data

Pass an `ActivitySnapshot` to the `activity` property. It binds the input to a
logical chain, full file record ID and digest, path identity, revision identity
and resulting content identity. A mismatch hides the supplied marks. Equal
content in a different revision cannot transfer attribution or exposure.

The provider supplies a complete resulting UTF-8 snapshot and `ActivityIndicator`
values. Each indicator has an explicit kind, plain-text label, exact supporting
`ActivitySource` records, and an optional half-open byte range. Providers own
author classification, edit replay, UTF-16 conversion and query completeness.
Missing, overlapping or contradictory classifications must remain explicit in
the supplied data. The component does not parse raw records or guess author kinds
from names, recorder IDs, message roles or tool success.

These types are local Rust presentation inputs, like the existing host capability
inputs. They do not extend the app-core wire protocol. The current shared history
view supplies author IDs and file actions but has no typed author classes or
mapped ranges. Existing mounts therefore show those file-level facts immediately;
a host must supply `activity` to show classified code ranges. Production range
projection and transport stay with their host-tools/app-core owners. The example
supplies labelled development data through the same property.

Without additional input, the panel reports author classification as unknown and
shows loaded `View`, `Read`, and applied `Create`/`Change` observations. Related
observations must match the selected revision, path and resulting content. An
open tab, snapshot, save or proposed change does not count as exposure or touch.
This is a view of loaded observations, not an exhaustive history query. Use the
existing history paging and refresh controls to load more records.
Known file-level actions remain visible even when supplied range observations are
empty or unavailable; their range coverage stays unknown.

## Meaning and missing observations

- Human, AI and other attribution are explicit supplied categories. The recorder
  remains separate in the standard observation card.
- A range with multiple author categories displays each category and a patterned
  color. No single author is chosen. File-level marks never color every byte.
- Exposure and recorded read intervals are positive observations, not claims of
  review or comprehension. Touch means an observed applied edit; it does not
  identify a human by itself.
- Unmarked bytes have unknown attribution, exposure and touch. Empty indicator
  lists do not establish unread or untouched code. The panel does not calculate
  an AI percentage that could conceal unknown coverage.
- Invalid, reversed, empty, out-of-bounds and split-UTF-8 ranges are excluded.
  Conflicted, missing or replaced source records retract their marks when those
  lookup results arrive; the source records remain inspectable.
- A missing snapshot differs from a recorded empty snapshot. When an exact
  `FileAfter` lookup is available, it must agree with the supplied text. Before
  snapshots are never painted with resulting-revision ranges.
- Loading and failed inputs do not retain their supplied marks as current data.
  Provider issues, including capture gaps and bounded scans, remain visible.

## Interaction and sources

Each heatmap range is a keyboard-operable button with a text description of its
byte interval, author categories, exposure and touch. Selecting it narrows the
observation and source lists. Unknown areas remain selectable. “Show all activity
sources” restores the full list; changing the selected chain or record resets
range selection. Rendering preserves the complete text, including Unicode and
line endings, in a bounded scroll region.

Source disclosure reuses the exact history record/Original panels. Inline loads
dispatch `LoadOperationDetails` and keep the selected file in place. Native
record opens dispatch `Open` with the full supplied record ID and digest; native
Original opens use the loaded Original's own record identity. Capability
revocation and pending opens use the existing controls. These actions remain
scoped to the current history chain and require the existing host adapters.

## Verification

```sh
./scripts/lint.sh
bash scripts/build-history-details.sh
CHROME_PATH=/path/to/chrome npm run test:history-details:browser
```

The history details gallery has a `src/greet.rs` file fixture with human, AI and
unknown ranges, observed exposure and touch, and a disclosed capture gap. Its
controls exercise another revision, empty observations, failed activity loads
and a conflicted AI source. Tests cover byte boundaries, missing/empty snapshots,
revision isolation, source retractions, exact source/Original actions, escaped
labels, both host compositions, keyboard interaction and narrow layouts.
