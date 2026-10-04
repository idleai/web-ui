use app_core::{Core, Effect, Event, resources, workspace::WorkspaceMode};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, VirtualDom, dioxus_core, rsx};

use super::ResourceDirectory;

fn render(view: resources::ResourceViewModel, hosts: bool) -> String {
    let mut dom = VirtualDom::new_with_props(preview, (view, hosts));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn preview((view, hosts): (resources::ResourceViewModel, bool)) -> Element {
    rsx! { ResourceDirectory { view, hosts, onaction: move |_| {}, onexecute: move |_| {} } }
}

fn fixture() -> Core {
    let core = Core::new();
    let snapshot = resources::scripted::demo_snapshot(WorkspaceMode::Standalone).unwrap();
    for effect in core.process_event(Event::Resources(resources::Event::Connect(
        snapshot.context.clone(),
    ))) {
        if let Effect::Resource(mut request) = effect {
            let _effects = core
                .resolve(
                    &mut *request,
                    Ok(resources::ResourceResult::Snapshot(Box::new(
                        snapshot.clone(),
                    ))),
                )
                .unwrap();
        }
    }
    core
}

#[test]
fn published_details_and_capability_gates_are_visible() {
    let core = fixture();
    let mut view = core.view().resources;
    let hosts = render(view.clone(), true);
    assert!(hosts.contains("Install and serve"));
    assert!(hosts.contains("Controller") && hosts.contains("Running"));
    assert!(hosts.contains("Owner:"));
    let providers = render(view.clone(), false);
    assert!(providers.contains("Use for session-shared"));
    assert!(providers.contains("Local provider") && providers.contains("runtime-evo"));
    view.capabilities = resources::ResourceCapabilities::default();
    view.hosts.iter_mut().for_each(|host| host.actions.clear());
    view.packages
        .iter_mut()
        .for_each(|package| package.can_install = false);
    let unavailable = render(view, true);
    assert!(unavailable.contains("Runtime actions are unavailable"));
    assert!(unavailable.contains("disabled"));
}

#[test]
fn pending_requests_and_runtime_failures_do_not_render_success() {
    let core = fixture();
    let request = resources::ResourceRequest {
        request_id: "connect:1".into(),
        expires_at_ms: 10_000,
    };
    let effects = core.process_event(Event::Resources(resources::Event::Execute {
        request: request.clone(),
        mutation: resources::ResourceMutation::ConnectHost {
            host_id: "host-shared".into(),
        },
    }));
    let pending = render(core.view().resources, true);
    assert!(pending.contains("Waiting for an action result"));
    assert!(!pending.contains("Runtime confirmed completion"));
    for effect in effects {
        if let Effect::Resource(mut operation) = effect {
            let _effects = core
                .resolve(
                    &mut *operation,
                    Err(resources::ResourceError {
                        code: resources::ResourceErrorCode::Unavailable,
                        message: "The runtime disconnected.".into(),
                        retry: resources::ResourceRetryAdvice::QueryStatus,
                    }),
                )
                .unwrap();
        }
    }
    let failed = render(core.view().resources, true);
    assert!(failed.contains("The runtime disconnected.") && failed.contains("Check status"));
    assert!(!failed.contains("Runtime confirmed completion"));
}
