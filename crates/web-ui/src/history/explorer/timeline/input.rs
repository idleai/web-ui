//! Row focus and disclosure are independent from selection and editor opening.

use app_core::history::timeline::{Event, Row, Surface};
use dioxus::prelude::{Key, KeyboardData, MouseData};

pub(super) fn action(row: &Row, key: &Key) -> Option<Event> {
    let group = row.group.as_ref().filter(|group| {
        group.header
            && !group.expanded
            && (row.presentation.task_summary || row.presentation.label.is_empty())
    });
    let expanded = group
        .map(|_| false)
        .or_else(|| row.details.as_ref().map(|details| details.expanded));
    let toggle = || {
        group.map_or_else(
            || Event::Details {
                surface: Surface::Editor,
                occurrence: row.occurrence.clone(),
            },
            |group| Event::Toggle {
                surface: Surface::Editor,
                group: group.id.clone(),
            },
        )
    };
    let enter = *key == Key::Enter;
    let space = *key == Key::Character(" ".into());
    if (*key == Key::ArrowLeft && expanded == Some(true))
        || (*key == Key::ArrowRight && expanded == Some(false))
        || ((enter || space) && expanded.is_some())
    {
        Some(toggle())
    } else if enter || space {
        Some(Event::Select {
            surface: Surface::Editor,
            occurrence: row.occurrence.clone(),
            open: enter,
        })
    } else {
        None
    }
}

pub(super) fn row_key(data: &KeyboardData) -> bool {
    #[cfg(target_arch = "wasm32")]
    if let Some(event) = data.downcast::<web_sys::KeyboardEvent>() {
        use wasm_bindgen::JsCast as _;
        return event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|element| element.class_list().contains("idle-timeline-row"));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _: &KeyboardData = data;
    true
}

pub(super) fn focus_row(data: &MouseData) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast as _;
        if let Some(row) = data
            .downcast::<web_sys::MouseEvent>()
            .and_then(|event| event.target())
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|element| element.closest(".idle-timeline-row").ok().flatten())
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _focused = row.focus_with_options(&options);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _: &MouseData = data;
}

pub(super) fn navigate(data: &KeyboardData, key: &Key, page: usize) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast as _;
        let current = data
            .downcast::<web_sys::KeyboardEvent>()
            .and_then(|event| event.target())
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|element| element.closest(".idle-timeline-row").ok().flatten());
        let Some(current) = current else {
            return false;
        };
        let rows = current
            .closest(".idle-timeline-viewport")
            .ok()
            .flatten()
            .and_then(|viewport| viewport.query_selector_all(".idle-timeline-row").ok());
        let Some(rows) = rows else {
            return false;
        };
        let Some(index) = (0..rows.length()).find(|index| {
            rows.item(*index)
                .is_some_and(|row| row.is_same_node(Some(&current)))
        }) else {
            return false;
        };
        let Some(next) = destination(
            usize::try_from(index).unwrap_or(0),
            usize::try_from(rows.length()).unwrap_or(0),
            key,
            page,
        ) else {
            return false;
        };
        let element = u32::try_from(next)
            .ok()
            .and_then(|index| rows.item(index))
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok());
        element.is_some_and(|element| element.focus().is_ok())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _: (&KeyboardData, &Key, usize) = (data, key, page);
        false
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn destination(current: usize, count: usize, key: &Key, page: usize) -> Option<usize> {
    let last = count.checked_sub(1)?;
    let next = if *key == Key::ArrowUp {
        current.checked_sub(1)?
    } else if *key == Key::ArrowDown {
        current.saturating_add(1).min(last)
    } else if *key == Key::Home {
        0
    } else if *key == Key::End {
        last
    } else if *key == Key::PageUp {
        current.saturating_sub(page.max(1))
    } else if *key == Key::PageDown {
        current.saturating_add(page.max(1)).min(last)
    } else {
        return None;
    };
    (next != current).then_some(next)
}

#[cfg(test)]
mod tests {
    use super::{Key, action, destination};
    use app_core::history::timeline::{Event, Group};

    #[test]
    fn mounted_navigation_stops_at_edges_and_home_end_use_mounted_bounds() {
        assert_eq!(
            destination(0, 7, &Key::ArrowUp, 3),
            None,
            "no wrapping at first row"
        );
        assert_eq!(
            destination(6, 7, &Key::ArrowDown, 3),
            None,
            "no wrapping at last row"
        );
        assert_eq!(
            destination(3, 7, &Key::Home, 3),
            Some(0),
            "first mounted row"
        );
        assert_eq!(destination(3, 7, &Key::End, 3), Some(6), "last mounted row");
    }

    #[test]
    fn folded_task_enter_toggles_but_expanded_plain_anchor_opens() {
        let mut row = super::super::super::tests::native_window().rows.remove(0);
        row.details = None;
        row.group = Some(Group {
            id: "task".into(),
            count: 2,
            summary: String::new(),
            live: false,
            header: true,
            expanded: false,
            entry: "first".into(),
            exit: row.occurrence.clone(),
        });
        assert!(
            matches!(action(&row, &Key::Enter), Some(Event::Toggle { .. })),
            "folded task discloses without selection"
        );
        row.group.as_mut().unwrap().expanded = true;
        assert!(
            matches!(
                action(&row, &Key::Enter),
                Some(Event::Select { open: true, .. })
            ),
            "expanded plain anchor opens its record"
        );
        assert!(
            action(&row, &Key::ArrowLeft).is_none(),
            "an expanded task has no implicit item disclosure"
        );
    }
}
