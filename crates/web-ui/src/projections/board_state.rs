//! Retain native disclosure and focus state when a board reparents a task.

use std::{collections::BTreeSet, rc::Rc};

use app_core::projections::ProjectionView;
use dioxus::prelude::{
    EventHandler, MountedData, MountedEvent, ReadableExt, WritableExt, use_effect, use_reactive,
    use_signal,
};
use wasm_bindgen::JsCast as _;
use web_sys::{Element, HtmlElement};

pub(super) fn use_board_state(view: &ProjectionView) -> EventHandler<MountedEvent> {
    let mut mounted = use_signal(|| None::<Rc<MountedData>>);
    // Read the old DOM before Dioxus moves rows to their new status parents.
    let snapshot = mounted.peek().as_deref().and_then(Snapshot::capture);
    let _restore = use_effect(use_reactive((&view.kind, &view.rows), move |_| {
        if let Some(snapshot) = &snapshot
            && let Some(mounted) = mounted.peek().as_deref()
            && let Some(board) = mounted.downcast::<Element>()
        {
            snapshot.restore(board);
        }
    }));
    EventHandler::new(move |event: MountedEvent| mounted.set(Some(event.data())))
}

struct Snapshot {
    kind: String,
    expanded: BTreeSet<String>,
    focused: Option<(String, FocusTarget)>,
}

enum FocusTarget {
    Title,
    Records,
    Reference(String),
}

impl Snapshot {
    fn capture(mounted: &MountedData) -> Option<Self> {
        let board = mounted.downcast::<Element>()?;
        let expanded = elements(board, "[data-row-key]")
            .into_iter()
            .filter(|row| {
                row.query_selector(".idle-projection-records[open]")
                    .ok()
                    .flatten()
                    .is_some()
            })
            .filter_map(|row| row.get_attribute("data-row-key"))
            .collect();
        Some(Self {
            kind: board.get_attribute("data-projection-kind")?,
            expanded,
            focused: focused(board),
        })
    }

    fn restore(&self, board: &Element) {
        if board.get_attribute("data-projection-kind").as_ref() != Some(&self.kind) {
            return;
        }
        let rows = elements(board, "[data-row-key]");
        for row in &rows {
            if row
                .get_attribute("data-row-key")
                .is_some_and(|key| self.expanded.contains(&key))
                && let Ok(Some(details)) = row.query_selector(".idle-projection-records")
                && !details.has_attribute("open")
            {
                let _result = details.set_attribute("open", "");
            }
        }
        // A real focus change to another control takes precedence over restoration.
        if board
            .owner_document()
            .and_then(|document| document.active_element())
            .is_some_and(|active| active.tag_name() != "BODY")
        {
            return;
        }
        if let Some((key, target)) = &self.focused
            && let Some(row) = rows
                .iter()
                .find(|row| row.get_attribute("data-row-key").as_ref() == Some(key))
            && let Some(element) = target
                .find(row)
                .or_else(|| row.query_selector(".idle-projection-title").ok().flatten())
            && let Some(element) = element.dyn_ref::<HtmlElement>()
        {
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _result = element.focus_with_options(&options);
        }
    }
}

fn focused(board: &Element) -> Option<(String, FocusTarget)> {
    let active = board.owner_document()?.active_element()?;
    if !board.contains(Some(&active)) {
        return None;
    }
    let key = active
        .closest("[data-row-key]")
        .ok()??
        .get_attribute("data-row-key")?;
    let target = if active.class_list().contains("idle-projection-title") {
        FocusTarget::Title
    } else if active.tag_name() == "SUMMARY" {
        FocusTarget::Records
    } else {
        FocusTarget::Reference(
            active
                .closest("[data-projection-reference]")
                .ok()??
                .get_attribute("data-projection-reference")?,
        )
    };
    Some((key, target))
}

impl FocusTarget {
    fn find(&self, row: &Element) -> Option<Element> {
        match self {
            Self::Title => row.query_selector(".idle-projection-title").ok().flatten(),
            Self::Records => row
                .query_selector(".idle-projection-records > summary")
                .ok()
                .flatten(),
            Self::Reference(key) => elements(row, "[data-projection-reference]")
                .into_iter()
                .find(|link| link.get_attribute("data-projection-reference").as_ref() == Some(key))
                .and_then(|link| link.query_selector("button").ok().flatten()),
        }
    }
}

fn elements(root: &Element, selector: &str) -> Vec<Element> {
    let Ok(nodes) = root.query_selector_all(selector) else {
        return Vec::new();
    };
    (0..nodes.length())
        .filter_map(|index| nodes.item(index)?.dyn_into::<Element>().ok())
        .collect()
}
