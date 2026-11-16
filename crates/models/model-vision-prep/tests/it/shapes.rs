use model_vision_prep::JpegQuality;

#[test]
fn jpeg_quality_refuses_zero_and_over_100() {
    JpegQuality::new(0).unwrap_err();
    JpegQuality::new(101).unwrap_err();
    serde_json::from_str::<JpegQuality>("0").unwrap_err();
}
