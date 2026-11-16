use base_secret::SecretText;
use model_http::{
    AuthHeader, ContentType, Exchange, ExtraHeader, Framing, HeaderName, JsonBody, RawBody,
    RequestId, RouteRoot, Upload, UrlPath, Verb,
};

#[test]
fn a_request_id_is_checked_where_it_enters() {
    RequestId::new("req_01HX").unwrap();
    RequestId::new("").unwrap_err();
    RequestId::new("a b").unwrap_err();
    RequestId::new("x".repeat(129)).unwrap_err();
    serde_json::from_str::<RequestId>(r#""with space""#).unwrap_err();
}

#[test]
fn secrets_and_bodies_do_not_print() {
    {
        let auth = AuthHeader::Bearer(SecretText::new("sk-very-secret"));
        assert!(!format!("{auth:?}").contains("sk-very-secret"));
        assert_eq!(
            format!("{:?}", JsonBody("{\"a\":1}".into())),
            "JsonBody(<7 bytes>)"
        );
    };
    {
        let header = ExtraHeader {
            name: HeaderName("x-routing".into()),
            value: SecretText::new("tenant-secret"),
        };
        assert!(!format!("{header:?}").contains("tenant-secret"));
        let json = r#"{"name":"x-routing","value":"tenant-secret"}"#;
        assert_eq!(serde_json::from_str::<ExtraHeader>(json).unwrap(), header);
    };
    {
        let ex = Exchange {
            verb: Verb::PostJson,
            root: RouteRoot::Base,
            path: UrlPath("/chat/completions".into()),
            body: Some(JsonBody(r#"{"prompt":"private"}"#.into())),
            framing: Framing::Sse,
        };
        assert!(!format!("{ex:?}").contains("private"));
        assert_eq!(ex.clone(), ex);
    };
    {
        let up = Upload {
            root: RouteRoot::Base,
            path: UrlPath("/audio/transcriptions".into()),
            body: RawBody {
                content_type: ContentType("multipart/form-data; boundary=b".into()),
                bytes: b"RIFF private speech".to_vec(),
            },
            framing: Framing::Whole,
        };
        let shown = format!("{up:?}");
        assert!(shown.contains("RawBody(<19 bytes>)"), "{shown}");
        assert!(!shown.contains("RIFF") && !shown.contains("private"));
        assert_eq!(up.clone(), up);
    }
}
