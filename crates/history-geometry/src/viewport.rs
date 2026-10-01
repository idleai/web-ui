//! Distinct coordinates at the protocol, visibility, and DOM boundaries.

/// Fixed row height in CSS pixels.
pub const ROW_H: i64 = 34;
/// Every row's pixel offset must remain an exact JavaScript integer too.
pub const MAX_RENDER_ROWS: i64 = 9_007_199_254_740_991 / ROW_H;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Row in the fully expanded legacy sequence.
pub struct ExpandedRow(i64);

impl ExpandedRow {
    /// Normalize a coordinate at the browser boundary.
    #[must_use]
    pub fn new(value: i64) -> Option<Self> {
        (0..=MAX_RENDER_ROWS)
            .contains(&value)
            .then_some(Self(value))
    }

    /// Exact integer coordinate.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Row in the visible legacy sequence.
pub struct VisibleRow(i64);

impl VisibleRow {
    /// Normalize a coordinate at the browser boundary.
    #[must_use]
    pub fn new(value: i64) -> Option<Self> {
        (0..=MAX_RENDER_ROWS)
            .contains(&value)
            .then_some(Self(value))
    }

    /// Exact integer coordinate.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }

    /// Fixed-height vertical position.
    #[must_use]
    pub fn pixel_offset(self) -> Pixels {
        Pixels(self.0.saturating_mul(ROW_H))
    }
}

/// Nonnegative CSS measurements normalized at the browser boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pixels(i64);

impl Pixels {
    /// Normalize a coordinate at the browser boundary.
    #[must_use]
    pub fn new(value: i64) -> Self {
        Self(value.clamp(0, MAX_RENDER_ROWS.saturating_mul(ROW_H)))
    }

    /// Exact integer coordinate.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }

    /// Visible row containing this position.
    #[must_use]
    pub fn row(self) -> VisibleRow {
        VisibleRow(self.0.saturating_div(ROW_H))
    }
}

/// Convert an exact, bounded browser coordinate to CSS pixels.
#[must_use]
pub fn i64_to_f64(value: i64) -> f64 {
    let high = i32::try_from(value >> 32).unwrap_or(0);
    let low = u32::try_from(value & 0xffff_ffff_i64).unwrap_or(0);
    f64::from(high) * 4_294_967_296.0 + f64::from(low)
}

/// Round a nonnegative CSS measurement into the exact browser integer range.
#[must_use]
pub fn pixel_integer(value: f64) -> i64 {
    const MAX_EXACT: i64 = 9_007_199_254_740_991;
    let target = if value.is_finite() {
        value.round().clamp(0.0, i64_to_f64(MAX_EXACT))
    } else {
        0.0
    };
    let mut low = 0_i64;
    let mut high = MAX_EXACT;
    while low < high {
        let mid = low.saturating_add(high.saturating_sub(low).saturating_div(2));
        if i64_to_f64(mid) < target {
            low = mid.saturating_add(1);
        } else {
            high = mid;
        }
    }
    low
}
