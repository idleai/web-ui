use app_core::{
    repository::{
        ReadReport, ReadState, RecordedSession, RepositoryContext, RepositoryLoadState,
        RepositoryScope, RepositorySnapshot, SourceRecord, ViewModel,
    },
    subscriptions, workspace,
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, VirtualDom, dioxus_core, rsx};

use super::{RecordedSessions, RepositoryOverview, RepositoryUsers};

fn render(view: ViewModel, screen: u8) -> String {
    let mut dom = VirtualDom::new_with_props(preview, (view, screen));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn preview((view, screen): (ViewModel, u8)) -> Element {
    match screen {
        0 => {
            rsx! { RepositoryOverview { view, onaction: move |_| {}, onopen: None, now_ms: 60_000 } }
        }
        1 => {
            rsx! { RepositoryUsers { view, workspace: workspace::ViewModel::default(), onaction: move |_| {}, onopen: None, now_ms: 60_000 } }
        }
        _ => rsx! { RecordedSessions { view, onaction: move |_| {}, now_ms: 60_000 } },
    }
}

fn fixture() -> ViewModel {
    ViewModel {
        context: Some(RepositoryContext {
            connection: subscriptions::Context {
                provider: "local".into(),
                workspace: "workspace".into(),
                contributor: "member".into(),
                chain: "chain".into(),
            },
            repository_id: "repository".into(),
        }),
        load: RepositoryLoadState::Ready,
        snapshot: Some(RepositorySnapshot {
            scope: RepositoryScope {
                workspace_id: "workspace".into(),
                repository_id: "repository".into(),
                chain: "chain".into(),
            },
            checked_at_ms: 1000,
            checkout: None,
            github: None,
            account: None,
            git_authors: vec![app_core::repository::GitAuthor {
                name: "<script>author</script>".into(),
                email: "author@example.test".into(),
                commits: 2,
            }],
            contributors: Vec::new(),
            collaborators: Vec::new(),
            sessions: Vec::new(),
            reports: vec![ReadReport {
                topic: "github.repository".into(),
                state: ReadState::Unavailable,
                message: "GitHub is offline; local history is available.".into(),
                checked_at_ms: 1000,
                retry_at_ms: Some(70_000),
                source_url: None,
            }],
        }),
        ..ViewModel::default()
    }
}

#[test]
fn offline_reads_keep_source_status_time_and_retry_visible() {
    let html = render(fixture(), 0);
    assert!(html.contains("GitHub is offline; local history is available."));
    assert!(html.contains("Checked 59s ago") && html.contains("retry in 10s"));
    assert!(
        html.contains("Refresh repository") && html.contains("Connect GitHub repository access")
    );
}

#[test]
fn repository_inspector_belongs_to_the_detail_view() {
    fn surface(surface: crate::assembly::Surface) -> Element {
        let mut view = app_core::ViewModel::default();
        view.workspace.selected_workspace = Some("workspace".into());
        view.repository = fixture();
        rsx! { crate::assembly::WorkspaceSurface {
            view, surface, capabilities: crate::host::HostCapabilities::new(crate::host::HostKind::VsCode),
            onaction: move |_| {},
        } }
    }
    for target in [
        crate::assembly::Surface::Sidebar,
        crate::assembly::Surface::Detail,
    ] {
        let mut dom = VirtualDom::new_with_props(surface, target);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert_eq!(
            html.contains("Refresh repository"),
            target == crate::assembly::Surface::Detail,
            "repository inspection remains available in the editor detail view"
        );
        assert_eq!(
            html.contains("Workspace navigation"),
            target == crate::assembly::Surface::Sidebar,
            "the sidebar retains the workspace overview"
        );
    }
}

#[test]
fn authorship_is_separate_from_idle_presence_and_markup_is_escaped() {
    let html = render(fixture(), 1);
    assert!(html.contains("No Idle members are available."));
    assert!(html.contains("author@example.test") && html.contains("2 commits in this read"));
    assert!(
        html.contains("GitHub contributors") && html.contains("Accessible GitHub collaborators")
    );
    assert!(!html.contains("<script>author</script>"));
}

#[test]
fn recorded_sessions_keep_full_identity_and_do_not_claim_runtime_availability() {
    let mut view = fixture();
    let id = "ab".repeat(32);
    view.selected_session = Some(id.clone());
    view.snapshot
        .as_mut()
        .expect("snapshot")
        .sessions
        .push(RecordedSession {
            id: id.clone(),
            labels: vec!["Imported conversation".into(), "Changed title".into()],
            actions: vec!["Started".into()],
            sources: vec!["importer".into()],
            records: vec![SourceRecord {
                observation: "cd".repeat(32),
                item: id.clone(),
                record_hash: "ef".repeat(32),
            }],
        });
    let html = render(view, 2);
    assert!(html.contains("No runtime connected") && html.contains("Imported conversation"));
    assert!(html.contains("Also recorded as: Changed title") && html.contains(&id));
    assert!(
        html.contains("Inspect session record 1") && html.contains("Show all recorded history")
    );
    assert!(html.contains("data-selected=\"true\""));
}
