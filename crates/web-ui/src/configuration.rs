//! Versioned settings and agent-rule forms over the shared editor state.

use app_core::configuration::{
    ConfigurationDocument, ConfigurationEditorAction, ConfigurationEditorView,
    ConfigurationLoadState, ConfigurationSaveState, ConfigurationViewModel, Event,
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, EventHandler, Props, component, dioxus_core, dioxus_elements, rsx};

use crate::controls::{Button, ControlState};

/// One independent document editor. Hosts allocate and persist a fresh save
/// identity in `onsave`; retries use the unchanged request retained by app-core.
/// Unknown document fields survive because the complete JSON text is edited.
#[component]
pub fn ConfigurationEditor(
    id: String,
    view: ConfigurationViewModel,
    document: ConfigurationDocument,
    onaction: EventHandler<Event>,
    onsave: EventHandler<ConfigurationDocument>,
) -> Element {
    let (title, editor) = match document {
        ConfigurationDocument::Settings => ("Settings", view.settings),
        ConfigurationDocument::AgentRules => ("Agent Rules", view.agent_rules),
    };
    let editable = editor.actions.contains(&ConfigurationEditorAction::Edit);
    let invalid = editor.validation_error.is_some();
    let reviewed_revision = editor.current.as_ref().map(|record| record.revision);
    let save_state = state(&editor, ConfigurationEditorAction::Save);
    let retry_state = state(&editor, ConfigurationEditorAction::RetrySave);
    let discard_state = state(&editor, ConfigurationEditorAction::Discard);
    let rebase_state = state(&editor, ConfigurationEditorAction::Rebase);
    rsx! {
        section { class: "idle-configuration idle-stack", aria_label: title,
            h2 { "{title}" }
            if document == ConfigurationDocument::AgentRules {
                p { "Saved repository rules. Enforcement requires a connected agent runtime." }
            }
            ConfigurationFeedback { editor: editor.clone() }
            if let Some(error) = &view.action_error { p { role: "alert", "{error.message}" } }
            div { class: "idle-field",
                label { r#for: "{id}-json", class: "idle-label", "{title} JSON" }
                textarea {
                    id: "{id}-json", class: "idle-input idle-configuration-input",
                    value: editor.draft.json.clone(), rows: 14,
                    readonly: !editable,
                    aria_invalid: invalid.to_string(), aria_describedby: "{id}-validation",
                    oninput: move |event| {
                        if editable { onaction.call(Event::Edit { document, json: event.value() }); }
                    },
                }
                p { id: "{id}-validation", class: "idle-field-message", "data-invalid": invalid.to_string(),
                    if let Some(error) = &editor.validation_error { "{error.message}" }
                    else { "Use a JSON object. Fields not recognized by this editor are preserved." }
                }
            }
            div { class: "idle-configuration-actions",
                Button { label: "Save", state: save_state, onpress: move |()| onsave.call(document) }
                if editor.pending.is_some() {
                    Button { label: "Recover save", state: retry_state, onpress: move |()| onaction.call(Event::RetrySave(document)) }
                }
                Button { label: "Discard changes", state: discard_state, onpress: move |()| onaction.call(Event::Discard(document)) }
                Button { label: "Refresh document", onpress: move |()| onaction.call(Event::Refresh) }
            }
            if editor.conflict {
                div { class: "idle-configuration-conflict",
                    p { role: "alert", "This document changed since you started editing. Review the saved value before applying your draft to its current revision." }
                    Button { label: "Use draft with current revision", state: rebase_state,
                        onpress: move |()| onaction.call(Event::Rebase { document, reviewed_revision }),
                    }
                }
            }
            details { class: "idle-configuration-current",
                summary { "Saved document" }
                if let Some(record) = editor.current {
                    p { "Revision {record.revision}" }
                    pre { code { "{record.value.json}" } }
                } else if editor.load == ConfigurationLoadState::Ready {
                    p { "This document has not been created." }
                } else { p { "The saved document has not been loaded." } }
            }
        }
    }
}

fn state(editor: &ConfigurationEditorView, action: ConfigurationEditorAction) -> ControlState {
    if editor.actions.contains(&action) {
        ControlState::Ready
    } else if action == ConfigurationEditorAction::Save
        && editor.save == ConfigurationSaveState::Saving
    {
        ControlState::Busy
    } else {
        ControlState::Disabled
    }
}

#[component]
fn ConfigurationFeedback(editor: ConfigurationEditorView) -> Element {
    rsx! {
        match &editor.load {
            ConfigurationLoadState::Idle => rsx! { p { "Select a repository to load this document." } },
            ConfigurationLoadState::Loading => rsx! { p { role: "status", "Loading document…" } },
            ConfigurationLoadState::Refreshing => rsx! { p { role: "status", "Refreshing the saved document…" } },
            ConfigurationLoadState::Suspended => rsx! { p { role: "status", "Connection interrupted. Your draft is retained while reconnecting." } },
            ConfigurationLoadState::Failed(error) => rsx! { p { role: "alert", "{error.message}" } },
            ConfigurationLoadState::Ready => rsx! {},
        }
        match &editor.save {
            ConfigurationSaveState::Idle => rsx! {},
            ConfigurationSaveState::Saving => rsx! { p { role: "status", "Saving document…" } },
            ConfigurationSaveState::Saved(revision) => rsx! { p { role: "status", "Saved revision {revision}." } },
            ConfigurationSaveState::Failed(error) => rsx! { p { role: "alert", "{error.message}" } },
            ConfigurationSaveState::Uncertain(error) => rsx! { p { role: "alert", "{error.message} Recover the original save before submitting another." } },
        }
        if editor.dirty { p { role: "status", "Unsaved changes" } }
        if !editor.actions.contains(&ConfigurationEditorAction::Edit) && editor.load == ConfigurationLoadState::Ready {
            p { "This document is read-only for the current connection." }
        }
    }
}

#[cfg(test)]
mod tests;
