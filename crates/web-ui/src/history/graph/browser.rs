//! Small DOM boundary; geometry and interaction decisions remain testable Rust.

use dioxus::prelude::{KeyboardData, MountedData, MouseData};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast as _;

pub(in crate::history) fn now() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.performance())
            .map_or(0.0, |clock| clock.now())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.0
    }
}

pub(in crate::history) fn scroll(mounted: &MountedData, top: f64) {
    #[cfg(target_arch = "wasm32")]
    if let Some(element) = mounted.downcast::<web_sys::Element>() {
        let options = web_sys::ScrollToOptions::new();
        options.set_top(top);
        options.set_left(f64::from(element.scroll_left()));
        options.set_behavior(web_sys::ScrollBehavior::Instant);
        if (f64::from(element.scroll_top()) - top).abs() > 0.5 {
            element.scroll_with_scroll_to_options(&options);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _: (&MountedData, f64) = (mounted, top);
    }
}

pub(in crate::history) fn focus(mounted: &MountedData) {
    #[cfg(target_arch = "wasm32")]
    if let Some(element) = mounted
        .downcast::<web_sys::Element>()
        .and_then(|element| element.dyn_ref::<web_sys::HtmlElement>())
    {
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _result = element.focus_with_options(&options);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _: &MountedData = mounted;
    }
}

pub(in crate::history) fn tree_key(data: &KeyboardData) -> bool {
    #[cfg(target_arch = "wasm32")]
    if let Some(event) = data.downcast::<web_sys::KeyboardEvent>() {
        return event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|element| element.class_list().contains("idle-history-viewport"));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _: &KeyboardData = data;
    true
}

pub(in crate::history) fn row_click(data: &MouseData) -> bool {
    #[cfg(target_arch = "wasm32")]
    if let Some(event) = data.downcast::<web_sys::MouseEvent>() {
        return event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .is_none_or(|element| {
                element
                    .closest("button,a,input,textarea,select,[data-history-action]")
                    .ok()
                    .flatten()
                    .is_none()
            });
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _: &MouseData = data;
    true
}
