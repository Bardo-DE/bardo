use vision_prep::JpegQuality;

#[test]
fn jpeg_quality_refuses_zero_and_over_100() {
    assert!(JpegQuality::new(0).is_err());
    assert!(JpegQuality::new(101).is_err());
    assert!(serde_json::from_str::<JpegQuality>("0").is_err());
}
