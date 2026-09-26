//! Controlled, native HTML controls. Values and semantic state belong to callers.

mod actions;
mod fields;

pub use actions::{
    Button, ButtonProps, ButtonVariant, Disclosure, DisclosureProps, IconButton, IconButtonProps,
};
pub use fields::{
    Checkbox, CheckboxProps, FieldMessage, FieldState, Select, SelectOption, SelectProps,
    TextField, TextFieldKind, TextFieldProps,
};

/// Availability of an action, supplied by its owner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlState {
    /// Accept user input.
    #[default]
    Ready,
    /// Unavailable and omitted from the native tab order.
    Disabled,
    /// Pending; buttons retain focus while suppressing repeat activation.
    Busy,
}

impl ControlState {
    pub(crate) const fn blocked(self) -> bool {
        !matches!(self, Self::Ready)
    }
}
