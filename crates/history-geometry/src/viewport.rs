//! Exact browser coordinate conversion.

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
