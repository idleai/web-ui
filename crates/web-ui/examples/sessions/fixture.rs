//! Development-only host adapter driving the real app-core reducer.

use app_core::sessions::{
    Event, SessionAcknowledgement, SessionAction, SessionCapabilities, SessionChange,
    SessionChangeEvent, SessionChanges, SessionContributor, SessionError, SessionErrorCode,
    SessionGrant, SessionGrantStatus, SessionInputRef, SessionInputState, SessionInputUpdate,
    SessionMutation, SessionOutput, SessionReceipt, SessionRequestKey, SessionResult,
    SessionRetryAdvice, SessionSnapshot, ViewModel,
    scripted::{ScriptedSessions, SessionScriptStep, demo_snapshot},
};
use app_core::workspace::WorkspaceMode;
use app_core::{Core, Effect, Event as AppEvent};

/// Explicit failure responses selectable from the development preview.
#[derive(Clone, Copy, Debug)]
pub(super) enum Reply {
    Receipt,
    Retryable,
    Uncertain,
    Commit,
}

/// Keeps shell continuations pending until a fixture control supplies a response.
pub(super) struct Fixture {
    core: Core,
    snapshot: SessionSnapshot,
    pending: Vec<Effect>,
    changes: Vec<SessionChange>,
    last_input: Option<SessionInputRef>,
}

impl Fixture {
    pub(super) fn new(mode: WorkspaceMode) -> Result<Self, String> {
        let mut snapshot =
            demo_snapshot(mode, "contributor-alice").map_err(|error| error.message)?;
        snapshot.inputs.push(SessionInputUpdate {
            input: SessionInputRef {
                session_id: "session-shared".into(),
                request: SessionRequestKey {
                    workspace_id: snapshot.context.workspace_id.clone(),
                    contributor_id: "contributor-bob".into(),
                    request_id: "remote-bob".into(),
                },
            },
            contributor: demo_snapshot(mode, "contributor-bob")
                .map_err(|error| error.message)?
                .contributor,
            runtime_id: "runtime-evo".into(),
            revision: 1,
            state: SessionInputState::Ordered(delivery(42)),
        });
        let mut fixture = Self {
            core: Core::new(),
            snapshot,
            pending: Vec::new(),
            changes: Vec::new(),
            last_input: None,
        };
        fixture.dispatch(Event::Connect(fixture.snapshot.context.clone()))?;
        fixture.dispatch(Event::Select(Some("session-shared".into())))?;
        Ok(fixture)
    }

    pub(super) fn view(&self) -> ViewModel {
        self.core.view().sessions
    }

    pub(super) fn dispatch(&mut self, event: Event) -> Result<(), String> {
        let effects = self.core.process_event(AppEvent::Sessions(event));
        self.accept(effects)
    }

    fn accept(&mut self, effects: Vec<Effect>) -> Result<(), String> {
        for effect in effects {
            if let Effect::Session(request) = effect {
                match &request.operation.action {
                    SessionAction::Snapshot => {
                        self.pending.retain(|effect| !matches!(effect, Effect::Session(request) if matches!(request.operation.action, SessionAction::Watch { .. })));
                        self.resolve(
                            Effect::Session(request),
                            Ok(SessionResult::Snapshot(Box::new(self.snapshot.clone()))),
                        )?;
                    }
                    SessionAction::RequestStatus(_) => {
                        self.resolve(Effect::Session(request), Ok(SessionResult::Unknown))?;
                    }
                    SessionAction::InputStatus(input) => {
                        let result = self
                            .snapshot
                            .inputs
                            .iter()
                            .find(|update| update.input == *input)
                            .cloned()
                            .map(SessionResult::Input)
                            .ok_or_else(|| failure(SessionRetryAdvice::QueryStatus));
                        self.resolve(Effect::Session(request), result)?;
                    }
                    SessionAction::Mutate {
                        request: attribution,
                        mutation: SessionMutation::Submit { session_id, .. },
                    } => {
                        self.last_input = Some(SessionInputRef {
                            session_id: session_id.clone(),
                            request: attribution.key(),
                        });
                        self.pending.push(Effect::Session(request));
                    }
                    SessionAction::Mutate { .. } | SessionAction::Watch { .. } => {
                        self.pending.push(Effect::Session(request));
                    }
                }
            }
        }
        Ok(())
    }

    fn resolve(&mut self, effect: Effect, result: SessionOutput) -> Result<(), String> {
        if let Effect::Session(mut request) = effect {
            let mut script = ScriptedSessions::new([SessionScriptStep {
                operation: request.operation.clone(),
                result,
            }]);
            let output = script.execute(&request.operation);
            let effects = self
                .core
                .resolve(&mut *request, output)
                .map_err(|error| format!("Fixture response failed: {error}"))?;
            self.accept(effects)?;
        }
        Ok(())
    }

    pub(super) fn reply(&mut self, reply: Reply) -> Result<(), String> {
        let Some(index) = self.pending.iter().position(|effect| matches!(effect, Effect::Session(request) if matches!(request.operation.action, SessionAction::Mutate { .. }))) else {
            return Err("Submit a prompt or sharing change first.".into());
        };
        let effect = self.pending.remove(index);
        let result = if let Effect::Session(request) = &effect
            && let SessionAction::Mutate { request, mutation } = &request.operation.action
        {
            let receipt = SessionReceipt {
                request: request.key(),
                received_at_ms: 1000,
                retry_until_ms: 3000,
            };
            match reply {
                Reply::Receipt => Ok(SessionResult::Acknowledged(
                    SessionAcknowledgement::Received(receipt),
                )),
                Reply::Retryable => Err(failure(SessionRetryAdvice::SameRequest {
                    not_before_ms: Some(1100),
                })),
                Reply::Uncertain => Err(failure(SessionRetryAdvice::QueryStatus)),
                Reply::Commit => {
                    self.queue_sharing(mutation, &request.contributor)?;
                    let mut through = self.snapshot.cursor.clone();
                    through.position = through.position.saturating_add(1);
                    Ok(SessionResult::Acknowledged(
                        SessionAcknowledgement::Committed {
                            receipt,
                            committed_at_ms: 1000,
                            through,
                        },
                    ))
                }
            }
        } else {
            return Err("Expected a mutation continuation.".into());
        };
        self.resolve(effect, result)
    }

    fn queue_sharing(
        &mut self,
        mutation: &SessionMutation,
        contributor: &SessionContributor,
    ) -> Result<(), String> {
        let grant = match mutation {
            SessionMutation::Invite {
                session_id,
                grant_id,
                grantee,
                permissions,
                expires_at_ms,
            } => SessionGrant {
                id: grant_id.clone(),
                session_id: session_id.clone(),
                grantee: grantee.clone(),
                granted_by: contributor.contributor_id.clone(),
                permissions: permissions.clone(),
                expires_at_ms: *expires_at_ms,
                revision: 1,
                status: SessionGrantStatus::Active,
            },
            SessionMutation::Revoke { grant_id, .. } => {
                let mut grant = self
                    .snapshot
                    .grants
                    .iter()
                    .find(|grant| grant.id == *grant_id)
                    .cloned()
                    .ok_or("Unknown fixture grant")?;
                grant.revision = grant.revision.saturating_add(1);
                grant.status = SessionGrantStatus::Revoked {
                    at_ms: 1000,
                    by: contributor.contributor_id.clone(),
                };
                grant
            }
            SessionMutation::Create(_) | SessionMutation::Submit { .. } => {
                return Err("Commit expects a sharing mutation.".into());
            }
        };
        self.changes.push(SessionChange::Grant(grant));
        Ok(())
    }

    pub(super) fn report(&mut self, state: SessionInputState) -> Result<(), String> {
        let input = self
            .last_input
            .clone()
            .ok_or("Submit a local prompt first")?;
        let previous = self
            .snapshot
            .inputs
            .iter()
            .find(|update| update.input == input);
        let update = SessionInputUpdate {
            input,
            contributor: self.snapshot.contributor.clone(),
            runtime_id: "runtime-evo".into(),
            revision: previous.map_or(1, |update| update.revision.saturating_add(1)),
            state,
        };
        self.changes.push(SessionChange::Input(update));
        self.publish()
    }

    pub(super) fn publish(&mut self) -> Result<(), String> {
        let Some(index) = self.pending.iter().position(|effect| matches!(effect, Effect::Session(request) if matches!(request.operation.action, SessionAction::Watch { .. }))) else {
            return Err("Reconnect before publishing fixture changes.".into());
        };
        let effect = self.pending.remove(index);
        let Effect::Session(request) = &effect else {
            return Err("Expected session effect".into());
        };
        let SessionAction::Watch { after } = &request.operation.action else {
            return Err("Expected watch".into());
        };
        let after = after.clone();
        let mut through = after.clone();
        let mut events = Vec::new();
        for change in std::mem::take(&mut self.changes) {
            match &change {
                SessionChange::Grant(grant) => {
                    self.snapshot.grants.retain(|old| old.id != grant.id);
                    self.snapshot.grants.push(grant.clone());
                }
                SessionChange::Input(update) => {
                    self.snapshot.inputs.retain(|old| old.input != update.input);
                    self.snapshot.inputs.push(update.clone());
                }
                SessionChange::Session(_) | SessionChange::Member(_) => {}
            }
            through.position = through.position.saturating_add(1);
            events.push(SessionChangeEvent {
                position: through.position,
                change,
            });
        }
        self.snapshot.cursor = through.clone();
        self.resolve(
            effect,
            Ok(SessionResult::Changes(SessionChanges {
                after,
                through,
                events,
                now_ms: self.snapshot.now_ms,
            })),
        )
    }

    pub(super) fn disconnect_updates(&mut self) -> Result<(), String> {
        let Some(index) = self.pending.iter().position(|effect| matches!(effect, Effect::Session(request) if matches!(request.operation.action, SessionAction::Watch { .. }))) else {
            return Err("No connected fixture watch".into());
        };
        let effect = self.pending.remove(index);
        self.resolve(effect, Err(failure(SessionRetryAdvice::QueryStatus)))
    }

    pub(super) fn unavailable(&mut self) -> Result<(), String> {
        self.snapshot.capabilities = SessionCapabilities::default();
        self.dispatch(Event::Refresh)
    }

    pub(super) fn tick(&mut self) -> Result<(), String> {
        self.snapshot.now_ms = 1100;
        self.dispatch(Event::Tick(1100))
    }

    pub(super) fn now_ms(&self) -> u64 {
        self.snapshot.now_ms
    }
}

pub(super) fn delivery(order: u64) -> app_core::sessions::SessionDelivery {
    app_core::sessions::SessionDelivery {
        accepted_at_ms: 1000,
        order,
        ordered_at_ms: 1000,
    }
}

fn failure(retry: SessionRetryAdvice) -> SessionError {
    SessionError {
        code: SessionErrorCode::Unavailable,
        message: "Fixture connection interrupted".into(),
        retry,
    }
}
