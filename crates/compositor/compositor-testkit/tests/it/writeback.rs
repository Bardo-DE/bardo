//! The writeback decoding (P1-V): pure, no device. Raw buffers as the vkms writeback connector
//! hands them back, decoded.

use compositor_render::pass::CaptureFormat;
use compositor_testkit::writeback::{RawFrame, WritebackError, decode, format_named};

/// 2x2 XRGB8888 pixels of `grey`, rows padded to a pitch of 12 bytes.
fn grey_raw(grey: u8) -> Vec<u8> {
    let row = [grey, grey, grey, 0, grey, grey, grey, 0, 7, 7, 7, 7];
    [row, row].concat()
}

fn frame() -> RawFrame {
    RawFrame::new(2, 2, 12, CaptureFormat::Xrgb8888)
}

#[test]
fn decode_swaps_blue_first_to_rgba_and_skips_row_padding() {
    let bytes = [
        3, 2, 1, 0, 6, 5, 4, 0, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    ];
    let image = decode(
        &RawFrame::new(2, 2, 12, CaptureFormat::Xrgb8888),
        &bytes[..],
    );
    let image = image.expect("a frame");
    assert_eq!(image.get_pixel(0, 0).0, [1, 2, 3, 255]);
    assert_eq!(image.get_pixel(1, 0).0, [4, 5, 6, 255]);
}

#[test]
fn argb_keeps_its_alpha_and_xrgb_ignores_the_fourth_byte() {
    let bytes = [3, 2, 1, 40];
    let argb = decode(&RawFrame::new(1, 1, 4, CaptureFormat::Argb8888), &bytes).expect("argb");
    let xrgb = decode(&RawFrame::new(1, 1, 4, CaptureFormat::Xrgb8888), &bytes).expect("xrgb");
    assert_eq!(argb.get_pixel(0, 0).0[3], 40);
    assert_eq!(xrgb.get_pixel(0, 0).0[3], 255);
}

#[test]
fn a_malformed_buffer_is_refused_not_misread() {
    let short = decode(&frame(), &[0; 10]);
    assert!(matches!(short, Err(WritebackError::Short { have: 10, .. })));
    let narrow = decode(&RawFrame::new(2, 2, 4, CaptureFormat::Xrgb8888), &[0; 64]);
    assert!(matches!(narrow, Err(WritebackError::PitchTooSmall { .. })));
    let empty = decode(&RawFrame::new(0, 2, 4, CaptureFormat::Xrgb8888), &[]);
    assert_eq!(empty, Err(WritebackError::Empty));
}

#[test]
fn the_last_row_needs_no_trailing_padding() {
    // pitch 12, height 2, width 2: 12 + 8 = 20 bytes are enough.
    let image = decode(&frame(), &grey_raw(100)[..20]);
    image.unwrap();
}

#[test]
fn format_names_are_the_drm_fourccs() {
    assert_eq!(format_named("AR24"), Some(CaptureFormat::Argb8888));
    assert_eq!(format_named("RG16"), None);
}
