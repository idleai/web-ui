# Activity editor and sidebar

`HistoryExplorer` renders the full Activity editor as one continuous table with
Graph, Activity, Tags, Content and Date columns. `HistoryMini` renders a separate
sidebar composition. Both consume `ViewModel.timeline` from app-core and paint
native routing fragments from the versioned `idle-history` timeline contract.

## Ownership and bounds

The native host owns display occurrences, exact relationships, causal ordering,
task groups, searchable fields and abstract lane routing. Web-ui paints the
returned lanes directly; it keeps pixel sizes, focus and viewport anchors.
App-core owns exact selection, filters, Find, disclosure overrides, pending
requests and refresh coordination. Editor and mini windows have independent
viewport state.

An occurrence identifies one displayed activity, including an explicitly recorded
logical item whose selected revision can change. Recorded rows carry a complete
operation ID, stored record hash and current/retained source. Live Git rows carry
a repository identity and full commit OID. A folded group retains constituent
record addresses, entry and exit attachments, and a representative operation.

Editor requests default to 200 rows, mini requests to 40, and the host rejects
requests above 500. App-core bounds both compositions together to 2,000 cached
row summaries and 32 MiB, including retained match pages. Full documents are
loaded only by native content actions. Native index construction reports progress
and accepts cancellation; rebuilding keeps the last complete snapshot readable.
Initial construction labels the activity count as unknown. During rebuilding,
the displayed count explicitly belongs to the previous snapshot. Group counts
refer to the recorded members of each safe interval.

## Interaction

| Gesture | Full editor | Sidebar mini |
| --- | --- | --- |
| Single click | Select an ordinary row; expand its item details or folded task when available; open an individual file | Reveal the exact occurrence in the full editor |
| Double click | Inspect an ordinary row's complete JSON | No extra activation |
| Enter | Toggle an expandable row; otherwise select and open its recorded target | Reveal the selected occurrence in the full editor |
| Space | Toggle an expandable row; otherwise select it | Existing mini keyboard actions |
| Arrow keys | Move focus independently of selection; Left/Right control item disclosure | Move selection without opening the editor |
| Home / End | Focus the first / last mounted row | Existing mini keyboard actions |
| Task count in Tags | Expand or fold the task without selection or activation | Same typed task action |
| Item arrow in Activity | Select and toggle attached details independently of the task | Full editor action |
| Open Activity | Existing host command and route | Footer action for the full editor |

File changes open a recorded diff when both sides are available. File reads and
open events open their recorded file. Missing file content opens exact operation
JSON with an explanation. Double-clicking an ordinary row opens full JSON;
buttons and file rows keep their single-click action. Superseded previews cannot
replace a newer selection.

Git commits disclose rename-aware file children from their first-parent diff,
including additions and deletions against the empty side. Each child carries the
repository, commit, paths, object IDs, modes, status and binary flag. Native
opening validates the full descriptor and reads immutable objects. Binary changes
show a message. Commit inspection opens a read-only JSON document.
File Content shows the filename and its directory; status and recorded-file
annotations stay in Tags.

Attached details participate in native window pagination and inherit the parent
activity's date. They have stable identities and pass-through graph rails, with
no extra node. Task counts continue to count physical activities. Pending
controls show busy state and remain clickable; repeated toggles retain each
intent. Folding a selected member clears selection instead of selecting the
representative.

Native live groups start expanded. Completed groups and imported legacy groups
start folded. The native host folds only connected safe paths, preserving forks,
joins, file activity, Git attachments and protected outcomes. Manual choices
persist through refresh and rebinding the same chain in the active runtime.
Completion keeps an expanded group open while its members are being read.

Ctrl/Cmd+F focuses in-place Find. Enter submits a new query or advances an existing
one; Shift+Enter and Up move backward, Down moves forward when the draft matches
the settled query. Escape clears Find and selection while retaining the draft.
Previous and Next return focus to the input. Matches expose individual physical
activities inside folded tasks; clearing Find keeps those activities exposed.
A manual task toggle clears these temporary exposures. PageUp/PageDown move
focus through mounted rows and scrolling requests adjacent native windows.

Rows stay readable during a failed fetch, with a local Retry action. Scrolling
retains the first visible occurrence and its within-row offset through window
replacement and cache eviction. New activity is followed only at the newest
position; older positions retain context and show a New activity action.

## Layout and routing

The editor uses 34-pixel rows and the sidebar uses 28-pixel rows, with VS Code theme tokens. The full
editor has a sticky aligned header and responsive Tags widths and hidden Date below 400 pixels. The graph can still
scroll horizontally when explicitly widened.
Previews truncate in the row; recorded author/session context and recorded dates
remain available. Dates use UTC, matching the earlier Activity view; missing dates
display “Time not recorded”. Description summaries strip Markdown markup, skip empty wrappers and join
meaningful lines with a middle dot. Content uses semantic icons and an optional
recorded tool heading. Activity and Date use the original monospace sizes.
Recorded HTML is never inserted into the document, and description links do not navigate.

Native routing includes shared rails, bends, joins and paths with both endpoints
outside the window. Group and filter boundaries retain explicit continuations.
View options contains the activity filter and graph controls. The graph column
fits the returned lanes automatically and can be resized, horizontally panned
and zoomed from 10 to 40 pixels per lane. Changing width never replans topology. Selection uses the editor selection
colors without changing node size or rail width. Clipped sidebar lanes show continuation marks and an Open Activity
action, using the same routing fragments as the editor.

Turns use two quadratic halves, each in its endpoint lane color, with vertical
tangents at row boundaries and horizontal tangents at a node. The editor uses
the original ten lane colors, 1.4-pixel rails, four-pixel dots and folded capsules. A curve consumes
the rail half it replaces, so a turn has no square corner or dangling stub.
Passing turns clear the row's unrelated dot. The native layout reserves each
active source's column until a recorded end, keeping independent streams and
nested children separate while their ancestors are idle. Reopening a source
after its column has been reused attaches to its actual older record.

## Historical recordings

The host resolves explicit operation and logical-item links, provider activation
and completion records, converted old-address mappings and repository-qualified
Git objects. An explicitly bound retained archive can repair missing converted
metadata. Missing or ambiguous endpoints remain unresolved. Names and timestamps
do not establish relationships. A resumed child does not move an earlier
completion attachment beyond its recorded execution boundary.
The Git reader includes HEAD ancestry and commit references recorded for the
bound repository, including other branches. Newly recorded references load on
refresh. Resolving those objects leaves HEAD, refs and recorded operations alone.

## Integration and fixture checks

Use `web_ui::assets::STYLESHEET` for the complete stylesheet. Standalone hosts must
give the editor a definite containing height. Native sidebar hosts may stretch
the mini viewport; resize observations update the mounted rows and scroll bounds.
`HistoryMini.onopen` routes its footer to the full Activity destination.

The fixture JSON under `examples/history_explorer/` is exported by host-tools'
`activity-fixture` example. It includes ten child agents, three nested levels,
a join retaining the parent continuation and all ten terminals, folded task
paths, an individually visible file change and hundreds of paging rows. The UI
fixture changes selection and disclosure over those native rows; it does not
calculate relationships or lane assignments.

The development fixture also offers seven small recordings: a single chain,
parent and child, fork and join, two children, a nested child, independent streams,
and an edge passing an unrelated activity. The browser checks their expected
columns, curve tangents, clearance around unrelated dots, and matching rail
endpoints across every row seam in both editor and mini.

```sh
./scripts/check.sh
bash scripts/build-history-explorer.sh
CHROME_PATH=/path/to/chrome npm run test:history-explorer:browser
```

The browser suite checks compact column alignment, bounded mounted rows, exact
native activation, disclosure, Find, paging, retries, graph controls, themes and
mini/editor handoff. Screenshots are saved under `out/history-explorer-tests/`.
The raw-history graph and rich detail components remain available for their
existing callers; the Activity editor uses the native timeline table.
