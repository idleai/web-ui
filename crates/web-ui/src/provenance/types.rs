use app_core::history::{RecordRef, RequestState};

/// Half-open byte offsets in the exact resulting UTF-8 snapshot.
/// Hosts map recorded byte or UTF-16 coordinates before supplying these ranges.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteRange {
    /// Inclusive byte offset at a UTF-8 boundary.
    pub start: usize,
    /// Exclusive byte offset at a UTF-8 boundary.
    pub end: usize,
}

/// A supplied positive observation; absence always leaves activity unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActivityKind {
    /// Explicitly classified human author, independent of the recorder.
    Human,
    /// Explicitly classified AI author, independent of the recorder.
    Ai,
    /// Explicit tool or system author.
    Other,
    /// Missing, unsupported or unresolved author classification.
    Unknown,
    /// Recorded visibility; does not establish review or comprehension.
    Exposure,
    /// Recorded read interval; does not establish review or comprehension.
    Read,
    /// An observed applied edit; does not by itself identify a human author.
    Touch,
}

impl ActivityKind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Human => "Human attribution",
            Self::Ai => "AI attribution",
            Self::Other => "Other attribution",
            Self::Unknown => "Unknown attribution",
            Self::Exposure => "Exposure observed",
            Self::Read => "Read interval observed",
            Self::Touch => "Touch observed",
        }
    }
}

/// Exact supporting record, with optional logical and Original links.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActivitySource {
    /// Full observation ID and original encoding digest.
    pub record: RecordRef,
    /// Recorded logical identity, when supplied.
    pub item: Option<String>,
    /// Linked Original observation, when supplied; its bytes may be missing.
    pub original: Option<String>,
}

/// A classification or observed action already resolved by the history provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivityIndicator {
    /// Recorded category; never derived from a display label.
    pub kind: ActivityKind,
    /// Exact resulting-snapshot range; `None` means file-level only.
    pub range: Option<ByteRange>,
    /// Plain text describing the recorded author or action.
    pub label: String,
    /// Records supporting this category and range. Positive marks require sources.
    pub sources: Vec<ActivitySource>,
}

/// Presentation input for one exact file occurrence in one logical chain.
///
/// Providers own semantic classification, edit replay, range mapping and query
/// completeness. This is a Rust component input, not a new history wire schema.
/// Empty indicators mean no observations supplied, never known inactivity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivitySnapshot {
    /// Logical chain of every supplied source.
    pub chain: String,
    /// Selected file observation and its exact stored digest.
    pub record: RecordRef,
    /// Exact path identity, not a working-copy path.
    pub path: String,
    /// Recorded revision identity; equal bytes never identify another revision.
    pub revision: Option<String>,
    /// Resulting content identity, in the same representation as history views.
    pub content_id: Option<String>,
    /// Complete resulting UTF-8 snapshot; `None` means unavailable, not empty.
    pub text: Option<String>,
    /// Positive recorded categories and mapped ranges for this occurrence only.
    pub indicators: Vec<ActivityIndicator>,
    /// Capture gaps, bounded-query limits and other provider-reported omissions.
    pub issues: Vec<String>,
    /// Loading/failure feedback. Only ready data is presented as current activity.
    pub state: RequestState,
}
