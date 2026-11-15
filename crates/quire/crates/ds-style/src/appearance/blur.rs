//! Blur availability (design/03-COLOR.md section 17.1).

/// The `data-blur` value for whether the host could get a compositor blur for this surface.
///
/// `data-blur="on"` paints `--m-tint` (translucent); `data-blur="off"` paints
/// `--m-tint-solid` (alpha at least .94). The default is the safe one: solid (`false`).
pub const fn blur_word(blur: bool) -> &'static str {
    if blur { "on" } else { "off" }
}
