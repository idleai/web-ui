//! Interactive session components with development-only app-core host responses.

#[path = "sessions/fixture.rs"]
mod fixture;
#[path = "sessions/output.rs"]
mod output;

use app_core::history::{self, Event as HistoryEvent, OperationDetailsState, OperationDetailsView};
use app_core::sessions::{
    Event as SessionEvent, SessionCompletion, SessionInputState, SessionMutationId, ViewModel,
};
use app_core::workspace::WorkspaceMode;
use dioxus::prelude::*;
use dioxus_html as dioxus_elements;
use dioxus_ssr as _;
use history_geometry as _;
#[cfg(target_arch = "wasm32")]
use {js_sys as _, web_sys as _};

use fixture::{Fixture, Reply, delivery};
use web_ui::controls::{Button, SelectOption};
use web_ui::host::{HostCapabilities, HostKind};
use web_ui::sessions::{
    InvitationAccess, InvitationDraft, PromptComposer, PromptDraft, SessionActionToken,
    SessionConversation, SessionFeedback, SessionSharing,
};
use web_ui::theme::{Density, Theme, ThemeProvider};

/// Mount development-only fixtures without credentials or production adapters.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    dioxus_web::launch::launch(Gallery, Vec::new(), Vec::new());
}

/// Browser and extension compositions share the same reducer and component set.
#[component]
pub fn Gallery() -> Element {
    let mut fixture = use_signal(|| Fixture::new(WorkspaceMode::Standalone));
    let initial = fixture.read().as_ref().ok().map(Fixture::view);
    let mut history = use_signal(|| {
        initial
            .as_ref()
            .and_then(|view| view.selected_history.as_ref())
            .map(output::view)
            .unwrap_or_default()
    });
    let mut feedback = use_signal(|| "No action dispatched".to_owned());
    let dispatch = Callback::new(move |event: SessionEvent| {
        feedback.set(format!("{event:?}"));
        if let Ok(fixture) = &mut *fixture.write() {
            if let Err(error) = fixture.dispatch(event) {
                feedback.set(error);
            }
            fixture.view()
        } else {
            ViewModel::default()
        }
    });
    let control = Callback::new(move |action: FixtureAction| {
        if let Ok(fixture) = &mut *fixture.write() {
            let result = match action {
                FixtureAction::Reply(reply) => fixture.reply(reply),
                FixtureAction::Report(state) => fixture.report(state),
                FixtureAction::Publish => fixture.publish(),
                FixtureAction::Disconnect => fixture.disconnect_updates(),
                FixtureAction::Unavailable => fixture.unavailable(),
                FixtureAction::Tick => fixture.tick(),
            };
            if let Err(error) = result {
                feedback.set(error);
            }
        }
    });
    let history_action = move |event: HistoryEvent| {
        feedback.set(format!("{event:?}"));
        apply_history(&mut history.write(), event);
    };
    let view = fixture
        .read()
        .as_ref()
        .map_or_else(|_| ViewModel::default(), Fixture::view);
    let now_ms = fixture.read().as_ref().map_or(1000, Fixture::now_ms);
    let fixture_error = fixture.read().as_ref().err().cloned();
    rsx! {
        ThemeProvider { theme: Theme::Light,
            main { class: "session-gallery",
                h1 { "Shared sessions" }
                p { "Development fixture · scripted host responses through app-core · no live runtime" }
                div { class: "fixture-actions",
                    Button { label: "Receive prompt", onpress: move |()| control.call(FixtureAction::Reply(Reply::Receipt)) }
                    Button { label: "Accept prompt", onpress: move |()| control.call(FixtureAction::Report(SessionInputState::Accepted { at_ms: 1000 })) }
                    Button { label: "Assign runtime order 9", onpress: move |()| control.call(FixtureAction::Report(SessionInputState::Ordered(delivery(9)))) }
                    Button { label: "Run prompt", onpress: move |()| control.call(FixtureAction::Report(SessionInputState::Running { delivery: delivery(9), started_at_ms: 1000 })) }
                    Button { label: "Complete prompt", onpress: move |()| control.call(FixtureAction::Report(SessionInputState::Completed { delivery: delivery(9), completed_at_ms: 1000, outcome: SessionCompletion::Succeeded })) }
                    Button { label: "Fail delivery", onpress: move |()| control.call(FixtureAction::Reply(Reply::Retryable)) }
                    Button { label: "Uncertain delivery", onpress: move |()| control.call(FixtureAction::Reply(Reply::Uncertain)) }
                    Button { label: "Advance provider clock", onpress: move |()| control.call(FixtureAction::Tick) }
                    Button { label: "Commit sharing", onpress: move |()| control.call(FixtureAction::Reply(Reply::Commit)) }
                    Button { label: "Publish sharing", onpress: move |()| control.call(FixtureAction::Publish) }
                    Button { label: "Disconnect updates", onpress: move |()| control.call(FixtureAction::Disconnect) }
                    Button { label: "Remove adapters", onpress: move |()| control.call(FixtureAction::Unavailable) }
                    Button { label: "Select control session", onpress: move |()| { let _view = dispatch.call(SessionEvent::Select(Some("session-control".into()))); } }
                    Button { label: "Select shared session", onpress: move |()| { let _view = dispatch.call(SessionEvent::Select(Some("session-shared".into()))); } }
                }
                p { class: "fixture-action", "{feedback}" }
                if let Some(error) = fixture_error { p { role: "alert", "{error}" } }
                div { class: "fixture-compositions",
                    HostSurface { id: "browser", kind: HostKind::Browser, view: view.clone(), history: history(), now_ms, dispatch, onhistoryaction: history_action }
                    HostSurface { id: "extension", kind: HostKind::VsCode, view, history: history(), now_ms, dispatch, onhistoryaction: history_action }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
enum FixtureAction {
    Reply(Reply),
    Report(SessionInputState),
    Publish,
    Disconnect,
    Unavailable,
    Tick,
}

fn token(view: &ViewModel, id: String) -> Option<SessionActionToken> {
    Some(SessionActionToken {
        context: view.context.clone()?,
        session_id: view.selected.clone()?,
        mutation: SessionMutationId {
            request_id: id,
            expires_at_ms: 2000,
        },
    })
}

#[component]
fn HostSurface(
    id: String,
    kind: HostKind,
    view: ViewModel,
    history: history::ViewModel,
    now_ms: u64,
    dispatch: Callback<SessionEvent, ViewModel>,
    onhistoryaction: EventHandler<HistoryEvent>,
) -> Element {
    let mut generation = use_signal(|| 0_u64);
    let mut prompt = use_signal(|| {
        token(&view, format!("{id}-prompt-0")).map(|token| PromptDraft {
            token,
            text: String::new(),
        })
    });
    let mut invitation = use_signal(|| {
        token(&view, format!("{id}-sharing-0")).map(|token| InvitationDraft {
            token,
            grant_id: format!("{id}-grant-0"),
            grantee: String::new(),
            access: InvitationAccess::Observe,
            expires_at_ms: None,
        })
    });
    let prompt_id = id.clone();
    let send = move |event: SessionEvent| {
        let next = dispatch.call(event);
        if prompt.read().as_ref().is_some_and(|draft| {
            next.mutations.iter().any(|mutation| {
                mutation.request.mutation.request_id == draft.token.mutation.request_id
            })
        }) {
            let next_generation = generation().saturating_add(1);
            generation.set(next_generation);
            prompt.set(
                token(&next, format!("{prompt_id}-prompt-{next_generation}")).map(|token| {
                    PromptDraft {
                        token,
                        text: String::new(),
                    }
                }),
            );
        }
    };
    let sharing_id = id.clone();
    let share = move |event: SessionEvent| {
        let next = dispatch.call(event);
        if invitation.read().as_ref().is_some_and(|draft| {
            next.mutations.iter().any(|mutation| {
                mutation.request.mutation.request_id == draft.token.mutation.request_id
            })
        }) {
            let next_generation = generation().saturating_add(1);
            generation.set(next_generation);
            invitation.set(
                token(&next, format!("{sharing_id}-sharing-{next_generation}")).map(|token| {
                    InvitationDraft {
                        token,
                        grant_id: format!("{sharing_id}-grant-{next_generation}"),
                        grantee: String::new(),
                        access: InvitationAccess::Observe,
                        expires_at_ms: None,
                    }
                }),
            );
        }
    };
    let session_action = move |event| {
        let _view = dispatch.call(event);
    };
    let recipients = vec![SelectOption {
        value: "contributor-bob".into(),
        label: "Bob (contributor-bob)".into(),
        disabled: false,
    }];
    rsx! {
        ThemeProvider { theme: if kind == HostKind::Browser { Theme::Light } else { Theme::VsCode }, density: if kind == HostKind::Browser { Density::Comfortable } else { Density::Compact },
            section { id: id.clone(),
                h1 { if kind == HostKind::Browser { "Browser host" } else { "Extension host" } }
                SessionFeedback { view: view.clone(), onaction: session_action }
                SessionConversation { id: "{id}-conversation", view: view.clone(), history,
                    capabilities: HostCapabilities::new(kind), onaction: session_action, onhistoryaction, now_ms,
                }
                PromptComposer { id: "{id}-composer", view: view.clone(), draft: prompt(), onaction: send,
                    oninput: move |text| { if let Some(draft) = &mut *prompt.write() { draft.text = text; } },
                }
                SessionSharing { id: "{id}-sharing", view, draft: invitation(), recipients, now_ms,
                    onchange: move |draft| invitation.set(Some(draft)), onaction: share,
                }
            }
        }
    }
}

fn apply_history(view: &mut history::ViewModel, event: HistoryEvent) {
    match event {
        HistoryEvent::ToggleDisclosure(key) => {
            if let Some(item) = view.items.iter_mut().find(|item| item.key == key) {
                item.expanded = !item.expanded;
            }
        }
        HistoryEvent::Select(selected) => {
            view.selected_item = view
                .items
                .iter()
                .find(|item| Some(&item.key) == selected.item.as_ref())
                .cloned();
            view.selected = selected;
        }
        HistoryEvent::LoadOperationDetails { operation, .. } => {
            view.operation_details
                .retain(|details| details.operation != operation);
            view.operation_details.push(OperationDetailsView {
                operation,
                state: OperationDetailsState::Failed(app_core::module::EffectError {
                    message: "Exact record adapter is unavailable in this session fixture.".into(),
                }),
            });
        }
        HistoryEvent::LoadItem(_)
        | HistoryEvent::LoadMore
        | HistoryEvent::Refresh
        | HistoryEvent::Connect(_)
        | HistoryEvent::Disconnect
        | HistoryEvent::Reconnect
        | HistoryEvent::Suspend
        | HistoryEvent::SetFilter(_)
        | HistoryEvent::Search(_)
        | HistoryEvent::SearchMore
        | HistoryEvent::NavigateMatch(_)
        | HistoryEvent::ClearSelection
        | HistoryEvent::Open { .. }
        | HistoryEvent::Completed { .. }
        | HistoryEvent::Timeline(_) => {}
    }
}

use serde_json as _;
