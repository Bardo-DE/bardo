//! The narrowing number conversions the components share, each with its bound stated once.
//!
//! Layout is done in `f32` pixels, the host reports pointer positions in `f64`, and a list's rows
//! are counted in `usize`. Every conversion here is saturating (a negative or not-a-number value
//! gives 0 where the target is unsigned), which is what an `as` cast does; naming it keeps the
//! lint's reasons in one place.

/// A length or position from the host's `f64` to the `f32` every quire rect is held in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "pixel lengths and unit fractions are far inside f32's exact range"
)]
pub(crate) fn f32_of(value: f64) -> f32 {
    value as f32
}

/// A count or index of rows, tiles or glyphs as `f32` layout arithmetic takes it.
#[expect(
    clippy::cast_precision_loss,
    reason = "counts of rows, tiles and glyphs are far under 2^24, where f32 is exact"
)]
pub(crate) fn count_f32(count: usize) -> f32 {
    count as f32
}

/// A count or index as `f64`.
#[expect(
    clippy::cast_precision_loss,
    reason = "counts of cells and rows are far under 2^53, where f64 is exact"
)]
pub(crate) fn count_f64(count: usize) -> f64 {
    count as f64
}

/// A pixel size (or a bounded integer angle) as `f32`.
#[expect(
    clippy::cast_precision_loss,
    reason = "pixel sizes and fixed-point steps are far under 2^24, where f32 is exact"
)]
pub(crate) fn u32_f32(value: u32) -> f32 {
    value as f32
}

/// A small signed integer as `f32`.
#[cfg(test)]
#[expect(
    clippy::cast_precision_loss,
    reason = "the integers converted are far under 2^24, where f32 is exact"
)]
pub(crate) fn i32_f32(value: i32) -> f32 {
    value as f32
}

/// The whole number of a measure in `f32`, for an index or a count: the fraction is dropped, a
/// negative or not-a-number measure is 0 and one past `usize::MAX` stays there.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a saturating cast, which is the meaning wanted: negative and NaN are 0, the fraction is dropped"
)]
pub(crate) fn whole_usize(value: f32) -> usize {
    value as usize
}

/// [`whole_usize`] for a `u32`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a saturating cast, which is the meaning wanted: negative and NaN are 0, the fraction is dropped"
)]
pub(crate) fn whole_u32(value: f32) -> u32 {
    value as u32
}

/// [`whole_usize`] for a `u16`, as a fixed-point thousandths value or a width in pixels.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a saturating cast, which is the meaning wanted: negative and NaN are 0, the fraction is dropped"
)]
pub(crate) fn whole_u16(value: f32) -> u16 {
    value as u16
}

/// [`whole_usize`] for a `u8`, as a pixel size or a colour channel.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a saturating cast, which is the meaning wanted: negative and NaN are 0, the fraction is dropped"
)]
pub(crate) fn whole_u8(value: f32) -> u8 {
    value as u8
}

/// [`whole_u8`] for an `f64` colour channel.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a saturating cast, which is the meaning wanted: negative and NaN are 0, the fraction is dropped"
)]
pub(crate) fn byte_of(value: f64) -> u8 {
    value as u8
}
