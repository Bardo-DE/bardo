use model_openai_compat::RequestJson;

#[test]
fn request_bodies_do_not_print() {
    assert_eq!(
        format!("{:?}", RequestJson("{}".into())),
        "RequestJson(<2 bytes>)"
    );
}

#[test]
fn multipart_bodies_do_not_print_the_audio() {
    use model_openai_compat::MultipartBody;
    let body = MultipartBody {
        content_type: "multipart/form-data; boundary=x".into(),
        bytes: vec![1, 2, 3],
    };
    assert_eq!(format!("{body:?}"), "MultipartBody(<3 bytes>)");
}
