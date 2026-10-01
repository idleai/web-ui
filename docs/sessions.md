# Shared sessions

`web_ui::sessions` renders app-core's f24 session views in browser and extension
hosts. It owns presentation and local keyboard composition state. app-core owns
selection, grants, pending requests, runtime facts, recovery and history state.
Hosts own credentials, durable request allocation, networking and effect adapters.

| Component | Input and behavior |
| --- | --- |
| `SessionFeedback` | Snapshot/update failures, loading, action errors and explicit refresh. |
| `SessionConversation` | Selected session, attributed prompts, runtime status and bound message/tool history. |
| `PromptComposer` | Controlled multiline `PromptDraft`; emits an exact `Submit` intent. |
| `SessionSharing` | Recorded grants and a controlled `InvitationDraft`; emits invitations and revocations. |
| `ParticipantAttribution` | Actual submitting identity with separate current-user and owner labels. |

Load `theme::STYLESHEET`, `history::details::STYLESHEET` and
`sessions::STYLESHEET` in the host. Mount within `ThemeProvider`; choose a browser
palette/comfortable density or VS Code tokens/compact density explicitly. The
components do not acquire a VS Code API, inject styles, or infer capabilities from
the host kind. The existing `Scaffold` remains available to its current consumers.

## Host integration

1. Connect app-core to an authenticated session adapter. Pass `sessions::ViewModel`
   back after every event and effect result. Dispatch callbacks through
   `app_core::Event::Sessions` and execute `Effect::Session` in the host.
2. Reserve a unique `SessionMutationId` and construct a `SessionActionToken` from
   the exact `SessionContext` and selected directory session. Retain the token
   together with the controlled text or invitation fields. Each new action needs
   its own identity, including actions from separate mounts or devices.
3. Persist the identity, provider-aligned deadline, attribution and exact payload
   before forwarding the action into app-core. After app-core stores the mutation,
   clear the submitted draft and reserve the next identity. A validation failure
   must keep the original draft. Do not rotate IDs to retry an uncertain request.
4. On selection, account or provider changes, restore the draft for that exact
   context or supply `None`. A draft token from another context is hidden and
   cannot submit. Draft text remains editable during a temporary disconnect;
   sending requires current app-core actions and an available input adapter.
5. Supply sharing recipient choices from the authorized workspace member view.
   `SelectOption.value` is the stable contributor ID; its label may be a friendly
   name. Default access is observation only. Other choices explicitly add prompt
   submission or sharing management. Reserve a new grant ID for every invitation.
6. Pass a provider-aligned `now_ms` to show grant expiry and unlock delayed
   retries. With no clock, deadline-based retries stay unavailable. Keep app-core
   updated through its `Tick` event; it remains the authority for allowed actions.

The preview in `crates/web-ui/examples/sessions.rs` shows the composition and
callback wiring. Its request counter and response controls are development-only;
production hosts need durable unique IDs and connected adapters. Fixture support
is enabled on the **dev dependency** only.

## Delivery and execution

The prompt list keeps the order supplied by app-core. Vector position, request ID,
receipt timestamp, history hash and event cursor never become a delivery number.
Only `SessionInputState::delivery()` supplies the displayed **Runtime order**.

| Supplied state | Presentation |
| --- | --- |
| Local `Pending` | Pending delivery; awaiting runtime confirmation. |
| Coordination `Received` | Received by coordination; still awaiting runtime confirmation. |
| Runtime `Accepted` | Accepted; awaiting order. |
| Runtime `Ordered` | Supplied order; awaiting execution. |
| Runtime `Running` | Running with the same supplied order. |
| Runtime `Completed` | Explicit success, failure or cancellation. |
| Runtime `Rejected` | Rejection reason; no invented order. |
| Delivery `Failed` / `Unknown` | Delivery failure or uncertainty, separate from any runtime fact. |

`Retry` keeps the original ID and exact payload held by app-core. It is offered
only for `SameRequest` advice after the supplied deadline. `QueryStatus` and
unknown outcomes offer `Recover`; they do not resubmit. A runtime fact disables
delivery retry. Observers can still look up an uncertain request after losing
submission permission. `RefreshInput` asks for the runtime's current status through the
typed effect boundary. Components never retry, refresh, invite or submit on mount.

Prompt text is escaped and retains whitespace. Recovered inputs without text say
so explicitly. The contributor is never replaced with the session owner. Verified
identity source/subject, request identity and runtime producer/revision are
available in disclosures. Ownership does not itself enable prompt submission.

## Conversation history

Bind a history reducer to `selected_history.chain` and set
`history::Filter.session` to `selected_history.item`, the full logical session
identity. A directory session ID is not a history item or an observation ID.
Supply this view separately; app-core's session reducer does not populate history
automatically. Both the chain/filter binding and each rendered item's observations
must match. Read access must remain available in the session's supplied actions.
The same checks prevent a retained inspector from showing another session's data.

The conversation reuses `HistoryRow` and `HistoryDetails`, preserving all loaded
message bytes, distinct tool attempts/channels, incomplete prefixes, missing
content, author/recorder identities, explicit preview limits and exact-record
inspection. Expansion, record selection, paging and native opens remain typed
`history::Event` callbacks. Pass advertised `HostCapabilities` for native opens;
either host works with none installed.

Prompts and recorded output are separate sections. The current app-core contract
has no prompt-to-turn mapping, so the UI does not infer a timeline by combining
their IDs or timestamps. A finished recorded tool block cannot complete a prompt.

## Sharing

The form sends `Invite` with its reserved request/grant IDs and explicit session
permissions. Revocation sends `Revoke` with a fresh request ID and the existing
grant ID; app-core supplies the expected revision. Pending, committed, failed and
unknown changes remain visible. Grants change only when app-core returns updated
records, including after a committed acknowledgement.

The participant roster keeps the session owner, grantee and original inviter
separate. Issued, expired and revoked records remain distinguishable. An issued
grant label is a recorded state, not a claim about current membership or presence.
Session grants never imply compute, file, process or model permission.

## Preview and verification

Keep app-core and EditChain next to this checkout. Run:

```sh
./scripts/lint.sh
./scripts/check.sh
./scripts/build-sessions.sh
CHROME_PATH=/path/to/chrome npm run test:sessions:browser
python3 -m http.server 4175 --bind 127.0.0.1 --directory out/sessions
```

The fixture uses f24's `demo_snapshot` and `ScriptedSessions` behind the real Crux
session reducer. Browser and extension compositions observe the same returned
state, with independent controlled drafts. Fixture buttons return receipts,
runtime facts, sharing commits/stream records, errors and disconnects explicitly.
No button in the production components assigns order or simulates execution.

Rust checks cover both coordination modes and host kinds, all execution outcomes,
exact input, attribution, scope changes, permission/capability removal, safe retry
and bound history. Chrome scenarios cover both mounts, concurrent input identities,
receipt/order/execution transitions, grant propagation/revocation, delayed retry,
reconnect, stale drafts, IME/newline behavior, focus and narrow layouts. CI builds
the WASM fixture and runs the Chrome suite alongside both existing history suites.

f32 owns this module, stylesheet, fixture, documentation and additive Cargo/npm/CI
wiring. app-core schemas and reducers are unchanged. Production extension/browser
assembly remains f43/f60; live shared input and remote runtime integration remain
f14/f17/f62. The fixture checks verify the shared components, not a live Evo
connection or execution authorization.

Checked on 2026-10-01 against app-core `0ebae9d` and EditChain `3c75cf0` in isolated
dependency checkouts. `./scripts/lint.sh` reported `RESULT: PASS` (exit 0), and
`./scripts/check.sh` exited 0, including 68 Rust tests and native/WASM builds.
All 21 Chrome scenarios passed without skips: seven graph, six history details
and eight session scenarios. Desktop and narrow session screenshots were inspected.
No lint policy changes or new suppressions were needed.
