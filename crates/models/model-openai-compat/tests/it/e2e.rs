//! The whole stack once: `OpenAiCodec` and `Driver` over `HttpClient` against a loopback server
//! that speaks the chat-completions stream on a Unix socket.

use std::time::Duration;

use model_http::{
    AuthHeader, HttpEndpoint, HttpTarget, Proxy, Timeouts, UrlPath, WaitMs,
};
use model_openai_compat::{Flavor, OpenAiCodec};
use model_provider::Provider;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;

fn endpoint(path: std::path::PathBuf) -> HttpEndpoint {
    HttpEndpoint {
        target: HttpTarget::Unix(path),
        proxy: Proxy::Direct,
        base: UrlPath("/v1".into()),
        auth: AuthHeader::None,
        headers: vec![],
        timeouts: Timeouts {
            connect: WaitMs(2_000),
            first_byte: WaitMs(5_000),
            idle: WaitMs(5_000),
        },
    }
}

/// Answers one request with `response`, in `pieces` written a few milliseconds apart; returns
/// the request it read.
fn serve_once(response: Vec<Vec<u8>>) -> (std::path::PathBuf, tokio::task::JoinHandle<String>) {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("openai-compat-e2e-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("engine.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let _ = std::fs::remove_dir_all(dir);
        let mut raw = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = stream.read(&mut buf).await.unwrap();
            raw.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&raw).into_owned();
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let want = head
                    .to_ascii_lowercase()
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length: ").map(str::to_owned))
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if body.len() >= want {
                    break;
                }
            }
        }
        for piece in response {
            stream.write_all(&piece).await.unwrap();
            stream.flush().await.unwrap();
            tokio::time::sleep(Duration::from_millis(3)).await;
        }
        String::from_utf8_lossy(&raw).into_owned()
    });
    (path, task)
}

#[tokio::test(flavor = "current_thread")]
async fn describe_reads_the_models_over_the_socket() {
    let body = r#"{"data":[{"id":"holo","max_model_len":16384}]}"#;
    let pieces =
        vec![format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()];
    let (path, server) = serve_once(pieces);
    let provider = OpenAiCodec::new(Flavor::Vllm).provider(model_http_reqwest::client(endpoint(path)));
    let models = provider.describe().await.unwrap();
    assert_eq!(models[0].loaded_context.0, 16384);
    assert!(
        server
            .await
            .unwrap()
            .starts_with("GET /v1/models HTTP/1.1\r\n")
    );
}
