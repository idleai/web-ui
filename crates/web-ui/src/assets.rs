//! Styles bundled with the component version selected by Cargo.

/// Complete shared stylesheet in the component cascade order.
pub const STYLESHEET: &str = concat!(
    include_str!("../assets/theme.css"),
    "\n",
    include_str!("../assets/history.css"),
    "\n",
    include_str!("../assets/history-details.css"),
    "\n",
    include_str!("../assets/sessions.css"),
    "\n",
    include_str!("../assets/navigation.css"),
    "\n",
    include_str!("../assets/projections.css"),
    "\n",
    include_str!("../assets/configuration.css"),
    "\n",
    include_str!("../assets/resources.css"),
    "\n",
    include_str!("../assets/repository.css"),
);
