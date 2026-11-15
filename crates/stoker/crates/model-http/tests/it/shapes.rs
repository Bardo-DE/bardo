use model_http::{
    AuthHeader, ContentType, Exchange, ExtraHeader, Framing, HeaderName, JsonBody, RawBody,
    RequestId, RouteRoot, Secret, Upload, UrlPath, Verb,
};

#[test]
fn a_request_id_is_checked_where_it_enters() {
    assert!(RequestId::new("req_01HX").is_ok());
    assert!(RequestId::new("").is_err());
    assert!(RequestId::new("a b").is_err());
    assert!(RequestId::new("x".repeat(129)).is_err());
    assert!(serde_json::from_str::<RequestId>(r#""with space""#).is_err());
}

#[test]
fn secrets_and_bodies_do_not_print() {
    {
        let auth = AuthHeader::Bearer(Secret("sk-very-secret".into()));
        assert!(!format!("{auth:?}").contains("sk-very-secret"));
        assert_eq!(
            format!("{:?}", JsonBody("{\"a\":1}".into())),
            "JsonBody(<7 bytes>)"
        );
    }
    {
        let header = ExtraHeader {
            name: HeaderName("x-routing".into()),
            value: Secret("tenant-secret".into()),
        };
        assert!(!format!("{header:?}").contains("tenant-secret"));
        let json = r#"{"name":"x-routing","value":"tenant-secret"}"#;
        assert_eq!(serde_json::from_str::<ExtraHeader>(json).unwrap(), header);
    }
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
    }
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
