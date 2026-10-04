use app_core::configuration::{
    ConfigurationDocument, ConfigurationEditorAction, ConfigurationError, ConfigurationErrorKind,
    ConfigurationLoadState, ConfigurationSaveState, ConfigurationViewModel,
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, VirtualDom, dioxus_core, rsx};

use super::ConfigurationEditor;

fn render(view: ConfigurationViewModel, document: ConfigurationDocument) -> String {
    let mut dom = VirtualDom::new_with_props(preview, (view, document));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn preview((view, document): (ConfigurationViewModel, ConfigurationDocument)) -> Element {
    rsx! {
        ConfigurationEditor { id: "configuration-test", view, document,
            onaction: move |_| {}, onsave: move |_| {},
        }
    }
}

#[test]
fn independent_documents_preserve_unknown_fields_and_escape_text() {
    let mut view = ConfigurationViewModel::default();
    view.settings.load = ConfigurationLoadState::Ready;
    view.settings.actions = vec![
        ConfigurationEditorAction::Edit,
        ConfigurationEditorAction::Save,
    ];
    view.settings.draft.json = r#"{"extension":{"unknown":"<script>alert(1)</script>"}}"#.into();
    view.agent_rules.draft.json = r#"{"instructions":"review every patch"}"#.into();
    let settings = render(view.clone(), ConfigurationDocument::Settings);
    assert!(
        settings.contains("Settings JSON"),
        "settings has a distinct label"
    );
    assert!(
        settings.contains("unknown") && settings.contains("alert(1)"),
        "unknown document content is retained"
    );
    assert!(
        !settings.contains("<script>"),
        "document text cannot create markup"
    );
    assert!(
        !settings.contains("review every patch"),
        "rules stay in their own editor"
    );
    let rules = render(view, ConfigurationDocument::AgentRules);
    assert!(
        rules.contains("Agent Rules JSON"),
        "rules has a distinct label"
    );
    assert!(
        rules.contains("review every patch"),
        "rules retain their full text"
    );
    assert!(rules.contains("readonly"), "capabilities gate editing");
}

#[test]
fn conflicts_and_uncertain_saves_remain_visible() {
    let mut view = ConfigurationViewModel::default();
    view.settings.load = ConfigurationLoadState::Ready;
    view.settings.dirty = true;
    view.settings.conflict = true;
    view.settings.save = ConfigurationSaveState::Uncertain(ConfigurationError {
        kind: ConfigurationErrorKind::Unavailable,
        message: "The connection closed.".into(),
    });
    let html = render(view, ConfigurationDocument::Settings);
    assert!(
        html.contains("Unsaved changes"),
        "a draft stays visibly unsaved"
    );
    assert!(
        html.contains("Recover the original save"),
        "uncertainty is not success"
    );
    assert!(
        html.contains("Use draft with current revision"),
        "rebasing requires review"
    );
    assert!(
        !html.contains("Saved revision"),
        "no saved result is manufactured"
    );
}
