//! Explicit presentation-host capabilities, separate from domain authorization.
//!
//! A host advertises only adapters it has installed. Components emit typed intents
//! synchronously from the user gesture; the host performs the operation, validates
//! targets, and reports its outcome through the caller's state. These are local
//! Rust interfaces, not a versioned transport schema or a general command bridge.

use std::collections::BTreeSet;
use std::fmt;
use std::num::NonZeroU32;

use app_core::history::OpenTarget;
use dioxus::prelude::*;

use crate::controls::{Button, ControlState};

/// Consumer identity; this never implicitly enables an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostKind {
    /// Browser application with host-owned routes and browser APIs.
    Browser,
    /// VS Code webview with an extension-owned message bridge.
    VsCode,
}

/// Individually negotiated host actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HostCapability {
    /// Open an external HTTP(S) link using the host's URL policy.
    OpenExternal,
    /// Write plain text to the clipboard after a user gesture.
    CopyText,
    /// Open a file or immutable revision in a host-provided editor.
    OpenFile,
    /// Compare two explicitly supplied file targets.
    OpenDiff,
    /// Open the exact retained operation encoding through history effects.
    OpenRecord,
    /// Open the captured Original through history effects.
    OpenOriginal,
    /// Reveal a file in the host's explorer.
    RevealFile,
}

/// Immutable capability snapshot for a consumer, denied by default.
///
/// Rebuild this snapshot when adapters or permissions change. Availability is a
/// presentation hint, not authorization; hosts must recheck at execution time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostCapabilities {
    kind: HostKind,
    supported: BTreeSet<HostCapability>,
}

impl HostCapabilities {
    /// Start with no capabilities, for either host kind.
    #[must_use]
    pub const fn new(kind: HostKind) -> Self {
        Self {
            kind,
            supported: BTreeSet::new(),
        }
    }

    /// Advertise a capability backed by an installed host adapter.
    #[must_use]
    pub fn with(mut self, capability: HostCapability) -> Self {
        let _inserted = self.supported.insert(capability);
        self
    }

    /// Consumer identity, independent of the set of available actions.
    #[must_use]
    pub const fn kind(&self) -> HostKind {
        self.kind
    }

    /// Whether this particular adapter is advertised.
    #[must_use]
    pub fn supports(&self, capability: HostCapability) -> bool {
        self.supported.contains(&capability)
    }

    /// Whether a history native-open adapter is installed. Components dispatch
    /// `app_core::history::Event::Open`; the host resolves its full record
    /// reference through the selected chain and rechecks access at execution.
    #[must_use]
    pub fn supports_history(&self, target: OpenTarget) -> bool {
        self.supports(match target {
            OpenTarget::Record => HostCapability::OpenRecord,
            OpenTarget::Original => HostCapability::OpenOriginal,
            OpenTarget::File => HostCapability::OpenFile,
            OpenTarget::Diff => HostCapability::OpenDiff,
        })
    }

    /// Check a typed request before presenting or dispatching it.
    ///
    /// # Errors
    /// Returns [`HostError::Unavailable`] if its adapter is not advertised.
    pub fn check(&self, request: &HostRequest) -> Result<(), HostError> {
        let capability = request.capability();
        if self.supports(capability) {
            Ok(())
        } else {
            Err(HostError::Unavailable(capability))
        }
    }
}

/// One-based Unicode scalar position. Hosts convert to their native coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextPosition {
    /// One-based line number.
    pub line: NonZeroU32,
    /// One-based Unicode scalar column, not a byte or UTF-16 offset.
    pub column: NonZeroU32,
}

/// File identity interpreted by the host through its repository bindings.
///
/// Paths are repository-relative. Hosts validate traversal, repository access,
/// revision existence and editor coordinate conversion. No local absolute path,
/// connection, credentials or native URI is required by a component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTarget {
    /// Opaque repository identity supplied by application state.
    pub repository: String,
    /// Repository-relative path supplied by evidence or application state.
    pub path: String,
    /// Immutable revision identity; `None` explicitly requests the working copy.
    pub revision: Option<String>,
    /// Optional position to reveal within the resolved content.
    pub position: Option<TextPosition>,
}

/// Intent emitted by a component. The owning host performs all side effects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostRequest {
    /// Open an external URL. The host validates scheme and destination; never
    /// interpret this as a script, command URI or arbitrary native URI.
    OpenExternal {
        /// An HTTP(S) URL from supplied presentation data.
        url: String,
    },
    /// Copy supplied plain text. Do not pass credentials through UI properties.
    CopyText {
        /// Text to copy after the user activates the action.
        text: String,
    },
    /// Open the exact supplied working file or historical revision.
    OpenFile(FileTarget),
    /// Compare two resolved targets; hosts must not substitute current content
    /// when historical content is missing.
    OpenDiff {
        /// Original content.
        before: FileTarget,
        /// Changed content.
        after: FileTarget,
    },
    /// Reveal the supplied file in a host-provided explorer.
    RevealFile(FileTarget),
}

impl HostRequest {
    /// Adapter needed to execute this intent.
    #[must_use]
    pub const fn capability(&self) -> HostCapability {
        match self {
            Self::OpenExternal { .. } => HostCapability::OpenExternal,
            Self::CopyText { .. } => HostCapability::CopyText,
            Self::OpenFile(_) => HostCapability::OpenFile,
            Self::OpenDiff { .. } => HostCapability::OpenDiff,
            Self::RevealFile(_) => HostCapability::RevealFile,
        }
    }
}

/// Host outcome failures; success is `Result::Ok(())`, never inferred on dispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostError {
    /// The adapter is unavailable or was removed after rendering.
    Unavailable(HostCapability),
    /// The user cancelled the native operation.
    Cancelled,
    /// The host rejected the request; text must be safe to show to the user.
    Denied(String),
    /// Execution failed; text must exclude credentials and raw transport details.
    Failed(String),
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(capability) => {
                write!(formatter, "Host capability {capability:?} is unavailable")
            }
            Self::Cancelled => formatter.write_str("Action cancelled"),
            Self::Denied(message) | Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for HostError {}

/// Capability-gated action with a visible, host-supplied unavailable reason.
///
/// Supply a stable document-unique ID. Unavailable actions remain visible and
/// disabled with an associated explanation. Dispatch preserves the original
/// request and does not claim completion. Hosts feed pending/error state back in.
#[component]
pub fn HostActionButton(
    id: String,
    label: String,
    unavailable_reason: String,
    capabilities: HostCapabilities,
    request: HostRequest,
    onrequest: EventHandler<HostRequest>,
    #[props(default)] state: ControlState,
) -> Element {
    let available = capabilities.check(&request).is_ok();
    rsx! {
        div { class: "idle-host-action",
            Button {
                label,
                state: if available { state } else { ControlState::Disabled },
                described_by: (!available).then(|| format!("{id}-unavailable")),
                onpress: move |()| {
                    if !state.blocked() && capabilities.check(&request).is_ok() {
                        onrequest.call(request.clone());
                    }
                },
            }
            if !available {
                p { id: "{id}-unavailable", class: "idle-field-message", "{unavailable_reason}" }
            }
        }
    }
}
