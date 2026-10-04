use std::collections::{BTreeMap, BTreeSet};

use app_core::history::{
    ContentValue, Detail, ObservationView, OperationDetailsState, RecordLookupStatus, RecordRef,
    RequestState, ViewModel,
};

use super::{ActivityIndicator, ActivityKind, ActivitySnapshot, ActivitySource, ByteRange};

#[derive(Clone, PartialEq)]
pub(super) struct Presentation {
    pub record: RecordRef,
    pub revision: Option<String>,
    pub text: Option<String>,
    pub indicators: Vec<ActivityIndicator>,
    pub unavailable_sources: BTreeSet<ActivitySource>,
    pub issues: Vec<String>,
    pub state: RequestState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Segment {
    pub range: ByteRange,
    pub kinds: BTreeSet<ActivityKind>,
}

impl Segment {
    pub(super) fn tone(&self) -> &'static str {
        let authors: Vec<_> = self
            .kinds
            .iter()
            .filter(|kind| {
                matches!(
                    kind,
                    ActivityKind::Human
                        | ActivityKind::Ai
                        | ActivityKind::Other
                        | ActivityKind::Unknown
                )
            })
            .collect();
        match authors.as_slice() {
            [ActivityKind::Human] => "human",
            [ActivityKind::Ai] => "ai",
            [ActivityKind::Other] => "other",
            [] | [ActivityKind::Unknown] => "unknown",
            _ => "mixed",
        }
    }

    pub(super) fn label(&self) -> String {
        let mut labels: Vec<_> = self.kinds.iter().map(|kind| kind.label()).collect();
        if !self.kinds.iter().any(|kind| {
            matches!(
                kind,
                ActivityKind::Human
                    | ActivityKind::Ai
                    | ActivityKind::Other
                    | ActivityKind::Unknown
            )
        }) {
            labels.insert(0, ActivityKind::Unknown.label());
        }
        if !self.kinds.contains(&ActivityKind::Exposure)
            && !self.kinds.contains(&ActivityKind::Read)
        {
            labels.push("Exposure unknown");
        }
        if !self.kinds.contains(&ActivityKind::Touch) {
            labels.push("Touch unknown");
        }
        format!(
            "Bytes {}–{}: {}",
            self.range.start,
            self.range.end,
            labels.join(" · ")
        )
    }
}

pub(super) fn selected_file(view: &ViewModel) -> Option<&ObservationView> {
    let item = view
        .selected_item
        .as_ref()
        .filter(|item| Some(&item.key) == view.selected.item.as_ref())
        .or_else(|| {
            view.items
                .iter()
                .find(|item| Some(&item.key) == view.selected.item.as_ref())
        })?;
    let observation = item.observations.iter().find(|observation| {
        Some(&observation.record.operation) == view.selected.observation.as_ref()
    })?;
    matches!(observation.detail, Detail::File { .. }).then_some(observation)
}

pub(super) fn source_available(view: &ViewModel, record: &RecordRef) -> bool {
    let cached_problem = view
        .items
        .iter()
        .chain(view.selected_item.iter())
        .flat_map(|item| &item.observations)
        .any(|observation| {
            observation.record.operation == record.operation
                && (observation.record != *record || observation.problem.is_some())
        });
    if cached_problem {
        return false;
    }
    view.operation_details
        .iter()
        .filter(|entry| entry.operation == record.operation)
        .all(|entry| match &entry.state {
            OperationDetailsState::Ready(details) => {
                details.operation == record.operation
                    && details.status == RecordLookupStatus::Found
                    && details
                        .observation
                        .as_ref()
                        .is_some_and(|observation| observation.record == *record)
            }
            OperationDetailsState::Idle
            | OperationDetailsState::Loading
            | OperationDetailsState::Failed(_) => false,
        })
}

pub(super) fn prepare(
    view: &ViewModel,
    supplied: Option<&ActivitySnapshot>,
) -> Option<Presentation> {
    let chain = view.chain.as_ref()?;
    let observation = selected_file(view)?;
    let Detail::File {
        path,
        revision,
        after,
        ..
    } = &observation.detail
    else {
        return None;
    };
    let mut result = Presentation {
        record: observation.record.clone(),
        revision: revision.clone(),
        text: None,
        indicators: fallback(view, observation),
        unavailable_sources: BTreeSet::new(),
        issues: Vec::new(),
        state: RequestState::Ready,
    };
    if !source_available(view, &observation.record) {
        result.indicators.clear();
        result.issues.push(
            "The selected record is unavailable or awaiting verification. Activity is unknown."
                .into(),
        );
        return Some(result);
    }
    if let Some(data) = supplied {
        if chain != &data.chain
            || data.record != observation.record
            || data.path != *path
            || data.revision != *revision
            || data.content_id != *after
        {
            result.issues.push("Supplied activity belongs to a different chain, record or revision. Its marks are unavailable here.".into());
        } else {
            result.state = data.state.clone();
            result.issues.clone_from(&data.issues);
            if data.state == RequestState::Ready {
                let recorded_actions: Vec<_> = result
                    .indicators
                    .iter()
                    .filter(|indicator| indicator.kind != ActivityKind::Unknown)
                    .cloned()
                    .collect();
                result.indicators.clone_from(&data.indicators);
                for action in recorded_actions {
                    if !result.indicators.iter().any(|indicator| {
                        indicator.kind == action.kind
                            && indicator.range.is_none()
                            && indicator.sources == action.sources
                    }) {
                        result.indicators.push(action);
                    }
                }
                result.text.clone_from(&data.text);
                if after.is_none() || !snapshot_matches(view, data) {
                    result.text = None;
                    result.issues.push("The resulting snapshot is unavailable or does not match the supplied text. Range marks are unavailable.".into());
                }
            }
        }
    }
    let before = result.indicators.len();
    result.indicators.retain(|indicator| {
        let valid = !indicator.sources.is_empty()
            && indicator
                .sources
                .iter()
                .all(|source| source_available(view, &source.record))
            && indicator.range.is_none_or(|range| {
                result
                    .text
                    .as_ref()
                    .is_some_and(|text| valid_range(text, range))
            });
        if !valid {
            result
                .unavailable_sources
                .extend(indicator.sources.iter().cloned());
        }
        valid
    });
    if result.indicators.len() != before {
        result.issues.push("Some marks have unavailable sources or text, pending source checks, or invalid byte ranges and cannot be displayed.".into());
    }
    result.issues.push("Only supplied observations are shown. Missing observations do not establish unread or untouched code. Visibility and read intervals do not establish review or comprehension.".into());
    match &view.reconciliation {
        RequestState::Loading => result
            .issues
            .push("History is refreshing; displayed observations may be out of date.".into()),
        RequestState::Failed(error) => result.issues.push(format!(
            "History refresh failed: {}. Displayed observations may be out of date.",
            error.message
        )),
        RequestState::Idle | RequestState::Ready => {}
    }
    Some(result)
}

fn snapshot_matches(view: &ViewModel, data: &ActivitySnapshot) -> bool {
    view.operation_details
        .iter()
        .filter(|entry| entry.operation == data.record.operation)
        .all(|entry| {
            let OperationDetailsState::Ready(details) = &entry.state else {
                return false;
            };
            details
                .fields
                .iter()
                .filter(|field| field.field == "\"FileAfter\"")
                .all(|field| {
                    field.record == data.record
                        && matches!(&field.value, ContentValue::Available(bytes)
                if data.text.as_ref().is_some_and(|text| text.as_bytes() == bytes))
                })
        })
}

fn source(observation: &ObservationView) -> ActivitySource {
    ActivitySource {
        record: observation.record.clone(),
        item: Some(observation.item.clone()),
        original: observation.original.clone(),
    }
}

fn fallback(view: &ViewModel, selected: &ObservationView) -> Vec<ActivityIndicator> {
    let mut indicators = vec![ActivityIndicator {
        kind: ActivityKind::Unknown,
        range: None,
        label: selected.author.as_ref().map_or_else(
            || "Author not recorded".into(),
            |author| format!("Recorded author: {author}. Author kind is not supplied."),
        ),
        sources: vec![source(selected)],
    }];
    let observations: BTreeMap<_, _> = view
        .items
        .iter()
        .chain(view.selected_item.iter())
        .flat_map(|item| &item.observations)
        .chain(std::iter::once(selected))
        .map(|observation| (observation.record.clone(), observation))
        .collect();
    for observation in observations.values() {
        if observation.problem.is_some() || !same_revision(selected, observation) {
            continue;
        }
        let Detail::File { action, change, .. } = &observation.detail else {
            continue;
        };
        let kind = match action.as_str() {
            "View" => ActivityKind::Exposure,
            "Read" => ActivityKind::Read,
            "Create" | "Change" if change.as_deref() == Some("Applied") => ActivityKind::Touch,
            _ => continue,
        };
        indicators.push(ActivityIndicator {
            kind,
            range: None,
            label: format!("Recorded {action} action; range coverage is not supplied."),
            sources: vec![source(observation)],
        });
    }
    indicators
}

fn same_revision(selected: &ObservationView, other: &ObservationView) -> bool {
    if selected.record == other.record {
        return true;
    }
    match (&selected.detail, &other.detail) {
        (
            Detail::File {
                path,
                revision: Some(revision),
                after: Some(after),
                ..
            },
            Detail::File {
                path: other_path,
                revision: Some(other_revision),
                after: Some(other_after),
                ..
            },
        ) => path == other_path && revision == other_revision && after == other_after,
        _ => false,
    }
}

fn valid_range(text: &str, range: ByteRange) -> bool {
    range.start < range.end
        && range.end <= text.len()
        && text.is_char_boundary(range.start)
        && text.is_char_boundary(range.end)
}

pub(super) fn segments(text: &str, indicators: &[ActivityIndicator]) -> Vec<Segment> {
    let boundaries: BTreeSet<_> = [0, text.len()]
        .into_iter()
        .chain(
            indicators
                .iter()
                .filter_map(|indicator| indicator.range)
                .filter(|range| valid_range(text, *range))
                .flat_map(|range| [range.start, range.end]),
        )
        .collect();
    boundaries
        .iter()
        .zip(boundaries.iter().skip(1))
        .map(|(&start, &end)| Segment {
            range: ByteRange { start, end },
            kinds: indicators
                .iter()
                .filter(|indicator| {
                    indicator.range.is_some_and(|range| {
                        valid_range(text, range) && range.start <= start && range.end >= end
                    })
                })
                .map(|indicator| indicator.kind)
                .collect(),
        })
        .collect()
}
