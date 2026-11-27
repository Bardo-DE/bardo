//! The filtering proxy and the request heads it reads: head parsing as table rows, and the proxy
//! itself over a unix socket pair against a loopback listener, with names answered by an injected
//! resolver (no real DNS, no real network). The policy's own verdicts are tested in `sandbox-core`.

use sandbox::egress::{
    BadRequestError, Entry, Gate, Journal, MAX_LINE, Outcome, Request, Resolver, Target,
    parse_head, read_head,
};
use sandbox_core::egress::{
    EgressHostError, EgressPolicy, EgressRefusedError, Host, HostPattern, Ports,
};
use std::io::{self, Read, Write};
use std::net::{IpAddr, TcpListener};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn pattern(text: &str) -> HostPattern {
    HostPattern::parse(text).expect("pattern")
}

fn ip(text: &str) -> IpAddr {
    text.parse().expect("ip")
}

fn host(text: &str) -> Host {
    Host::parse(text).expect("host")
}

fn policy(allow: &[&str], deny: &[&str], ports: Ports) -> EgressPolicy {
    let list = |l: &[&str]| l.iter().map(|t| pattern(t)).collect();
    EgressPolicy::new(list(allow), list(deny), ports).expect("policy")
}

fn target(name: &str, port: u16) -> Target {
    Target {
        host: host(name),
        port,
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one table of request heads and the fault each must give: a flat list of cases"
)]
fn request_heads_parse_or_name_the_fault() {
    let long = format!(
        "GET http://{}.example.com/ HTTP/1.1\r\n\r\n",
        "a".repeat(MAX_LINE)
    );
    let tunnel = |name, port| Ok(Request::Tunnel(target(name, port)));
    let relay = |name, port| Ok(Request::Relay(target(name, port)));
    let rows: Vec<(String, Result<Request, BadRequestError>)> = vec![
        (
            "CONNECT api.example.com:443 HTTP/1.1\r\nHost: api.example.com:443\r\n\r\n".into(),
            tunnel("api.example.com", 443),
        ),
        (
            "CONNECT [2001:db8::1]:8443 HTTP/1.1\r\n\r\n".into(),
            tunnel("2001:db8::1", 8443),
        ),
        (
            "GET http://API.example.com/a/b?c=d HTTP/1.1\r\nHost: api.example.com\r\n\r\n".into(),
            relay("api.example.com", 80),
        ),
        (
            "GET http://api.example.com:8080/ HTTP/1.1\r\n\r\n".into(),
            relay("api.example.com", 8080),
        ),
        (
            "POST HTTP://api.example.com HTTP/1.0\r\n\r\n".into(),
            relay("api.example.com", 80),
        ),
        (
            "GET http://[::1]/ HTTP/1.1\r\n\r\n".into(),
            relay("::1", 80),
        ),
        (
            "CONNECT api.example.com HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::BadPort),
        ),
        (
            "CONNECT api.example.com:0 HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::BadPort),
        ),
        (
            "CONNECT api.example.com:99999 HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::BadPort),
        ),
        (
            "CONNECT api.example.com:+80 HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::BadPort),
        ),
        (
            "CONNECT api.example.com: HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::BadPort),
        ),
        (
            "CONNECT :443 HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::NoHost),
        ),
        (
            "GET http:///x HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::NoHost),
        ),
        (
            "CONNECT ::1:443 HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::Malformed),
        ),
        (
            "CONNECT [::1]x HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::Malformed),
        ),
        (
            "CONNECT [::1:443 HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::Malformed),
        ),
        (
            "CONNECT bad!host:443 HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::BadHost(EgressHostError::BadChar)),
        ),
        (
            "CONNECT 127.1:80 HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::BadHost(EgressHostError::Numeric)),
        ),
        (
            "GET /path HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::NotProxy),
        ),
        (
            "GET https://api.example.com/ HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::NotProxy),
        ),
        (
            "GET http://user@api.example.com/ HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::NotProxy),
        ),
        (
            "GET http://api.example.com/ HTTP/2\r\n\r\n".into(),
            Err(BadRequestError::Malformed),
        ),
        (
            "get http://api.example.com/ HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::Malformed),
        ),
        (
            "GET http://api.example.com/\r\n\r\n".into(),
            Err(BadRequestError::Malformed),
        ),
        (
            "CONNECT a b:443 HTTP/1.1\r\n\r\n".into(),
            Err(BadRequestError::Malformed),
        ),
        ("garbage\r\n\r\n".into(), Err(BadRequestError::Malformed)),
        (String::new(), Err(BadRequestError::Malformed)),
        (long, Err(BadRequestError::TooLong)),
    ];
    for (head, want) in rows {
        let shown: String = head.chars().take(60).collect();
        assert_eq!(parse_head(head.as_bytes()), want, "{shown:?}");
    }
    assert_eq!(
        parse_head(b"GET \xff HTTP/1.1\r\n\r\n"),
        Err(BadRequestError::Malformed)
    );
}

#[test]
fn reading_a_head_is_bounded_in_size_and_time() {
    // Early bytes after the blank line stay in `bytes`, past `end`.
    let (mut near, mut far) = UnixStream::pair().expect("pair");
    let text = "CONNECT h.example:443 HTTP/1.1\r\n\r\n";
    near.write_all(format!("{text}EARLY").as_bytes())
        .expect("write");
    let head = read_head(&mut far, Duration::from_secs(5)).expect("head");
    assert_eq!(&head.bytes[..head.end], text.as_bytes());
    assert!(b"EARLY".starts_with(&head.bytes[head.end..]));

    let (mut near, mut far) = UnixStream::pair().expect("pair");
    near.write_all(&vec![b'a'; MAX_LINE + 10]).expect("write");
    assert_eq!(
        read_head(&mut far, Duration::from_secs(5)),
        Err(BadRequestError::TooLong)
    );

    let (mut near, mut far) = UnixStream::pair().expect("pair");
    let line = "X-Pad: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\n";
    near.write_all(b"GET http://a.example/ HTTP/1.1\r\n")
        .expect("write");
    // One write: many small ones each cost a socket buffer slot and fill it before anyone reads.
    near.write_all(line.repeat(600).as_bytes()).expect("write");
    assert_eq!(
        read_head(&mut far, Duration::from_secs(5)),
        Err(BadRequestError::TooLong)
    );

    let (near, mut far) = UnixStream::pair().expect("pair");
    drop(near);
    assert_eq!(
        read_head(&mut far, Duration::from_secs(5)),
        Err(BadRequestError::Closed)
    );

    let (mut near, mut far) = UnixStream::pair().expect("pair");
    near.write_all(b"GET http://a").expect("write");
    drop(near);
    assert_eq!(
        read_head(&mut far, Duration::from_secs(5)),
        Err(BadRequestError::Truncated)
    );

    let (mut near, mut far) = UnixStream::pair().expect("pair");
    near.write_all(b"GET http://a").expect("write");
    assert_eq!(
        read_head(&mut far, Duration::from_millis(150)),
        Err(BadRequestError::Timeout)
    );
    drop(near);
}

/// Answers from a table and counts the lookups; a name not in it fails.
#[derive(Debug, Default)]
struct Table {
    rows: Vec<(&'static str, Vec<IpAddr>)>,
    calls: AtomicUsize,
}

impl Resolver for Table {
    fn resolve(&self, name: &str, _port: u16) -> io::Result<Vec<IpAddr>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.rows
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, a)| a.clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such name"))
    }
}

#[derive(Debug, Default)]
struct Recorder(Mutex<Vec<Entry>>);

impl Journal for Recorder {
    fn record(&self, entry: &Entry) {
        self.0.lock().expect("lock").push(entry.clone());
    }
}

struct Rig {
    gate: Gate,
    table: Arc<Table>,
    journal: Arc<Recorder>,
}

fn rig(port: u16) -> Rig {
    let table = Arc::new(Table {
        rows: vec![
            ("files.example.test", vec![ip("127.0.0.1")]),
            ("rebind.example.test", vec![ip("10.0.0.9")]),
            ("mixed.example.test", vec![ip("8.8.8.8"), ip("10.0.0.9")]),
        ],
        calls: AtomicUsize::new(0),
    });
    let journal = Arc::new(Recorder::default());
    let allow = [
        "files.example.test",
        "rebind.example.test",
        "mixed.example.test",
        "gone.example.test",
        // The stand-in server lives on the loopback; only this exact address may be reached.
        "127.0.0.1",
    ];
    let p = policy(&allow, &[], Ports::new(vec![port]).expect("ports"));
    let gate = Gate::new(
        p,
        Arc::<Table>::clone(&table),
        Arc::<Recorder>::clone(&journal),
    );
    Rig {
        gate,
        table,
        journal,
    }
}

fn timed(stream: &UnixStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("timeout");
}

fn listener() -> (TcpListener, u16) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|e| panic!("the egress proxy tests need a loopback interface: {e}"));
    let port = listener.local_addr().expect("addr").port();
    (listener, port)
}

#[test]
fn a_connect_tunnel_reaches_the_resolved_address_and_carries_early_bytes() {
    let (server, port) = listener();
    let rig = rig(port);
    let echo = std::thread::spawn(move || {
        let (mut conn, _) = server.accept().expect("accept");
        let mut buf = [0_u8; 4];
        conn.read_exact(&mut buf).expect("read");
        conn.write_all(&buf).expect("echo");
    });
    let (mut near, far) = UnixStream::pair().expect("pair");
    timed(&near);
    let gate = rig.gate.clone();
    let worker = std::thread::spawn(move || gate.serve(far));
    near.write_all(format!("CONNECT files.example.test:{port} HTTP/1.1\r\n\r\nping").as_bytes())
        .expect("write");
    let want = "HTTP/1.1 200 Connection established\r\n\r\n";
    let mut got = vec![0_u8; want.len()];
    near.read_exact(&mut got).expect("established");
    assert_eq!(got, want.as_bytes());
    let mut echoed = [0_u8; 4];
    near.read_exact(&mut echoed).expect("echo");
    assert_eq!(&echoed, b"ping");
    drop(near);
    echo.join().expect("server");
    worker.join().expect("serve ends");
    assert_eq!(rig.table.calls.load(Ordering::SeqCst), 1, "resolved once");
    let entries = rig.journal.0.lock().expect("lock").clone();
    assert_eq!(
        entries,
        [Entry {
            target: Some(target("files.example.test", port)),
            outcome: Outcome::Tunnelled
        }]
    );
}

#[test]
fn a_plain_request_is_forwarded_unchanged() {
    let (server, port) = listener();
    let rig = rig(port);
    let request = format!(
        "GET http://files.example.test:{port}/x?y=1 HTTP/1.1\r\nHost: files.example.test\r\n\r\n"
    );
    let seen = std::thread::spawn(move || {
        let (mut conn, _) = server.accept().expect("accept");
        let mut got = Vec::new();
        let mut chunk = [0_u8; 256];
        while !got.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = conn.read(&mut chunk).expect("read");
            assert!(n > 0, "the request ended early");
            got.extend_from_slice(&chunk[..n]);
        }
        conn.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .expect("reply");
        got
    });
    let (mut near, far) = UnixStream::pair().expect("pair");
    timed(&near);
    let gate = rig.gate.clone();
    let worker = std::thread::spawn(move || gate.serve(far));
    near.write_all(request.as_bytes()).expect("write");
    let mut reply = String::new();
    near.read_to_string(&mut reply).expect("reply");
    assert!(reply.starts_with("HTTP/1.1 200 OK"), "{reply:?}");
    assert!(reply.ends_with("ok"));
    drop(near);
    assert_eq!(seen.join().expect("server"), request.as_bytes());
    worker.join().expect("serve ends");
    assert_eq!(rig.table.calls.load(Ordering::SeqCst), 1, "resolved once");
}

#[test]
fn refusals_answer_in_plain_text_and_never_connect() {
    // The listener is never accepted on: a connect that got through would still succeed in the
    // kernel's queue, so the journal is what proves the verdict.
    let (_server, port) = listener();
    let rig = rig(port);
    let rows: Vec<(String, &str, Option<&str>, Outcome)> = vec![
        (
            format!("CONNECT other.example.test:{port} HTTP/1.1\r\n\r\n"),
            "HTTP/1.1 403 Forbidden",
            Some("other.example.test"),
            Outcome::Refused(EgressRefusedError::NotListed),
        ),
        (
            "CONNECT files.example.test:22 HTTP/1.1\r\n\r\n".to_owned(),
            "HTTP/1.1 403 Forbidden",
            Some("files.example.test:22"),
            Outcome::Refused(EgressRefusedError::Port),
        ),
        (
            format!("CONNECT 10.0.0.1:{port} HTTP/1.1\r\n\r\n"),
            "HTTP/1.1 403 Forbidden",
            Some("10.0.0.1"),
            Outcome::Refused(EgressRefusedError::NotListed),
        ),
        (
            format!("CONNECT rebind.example.test:{port} HTTP/1.1\r\n\r\n"),
            "HTTP/1.1 403 Forbidden",
            Some("rebind.example.test"),
            Outcome::Refused(EgressRefusedError::SpecialAddress),
        ),
        (
            format!("GET http://mixed.example.test:{port}/ HTTP/1.1\r\n\r\n"),
            "HTTP/1.1 403 Forbidden",
            Some("mixed.example.test"),
            Outcome::Refused(EgressRefusedError::SpecialAddress),
        ),
        (
            format!("CONNECT gone.example.test:{port} HTTP/1.1\r\n\r\n"),
            "HTTP/1.1 502 Bad Gateway",
            Some("gone.example.test"),
            Outcome::Unreachable,
        ),
        (
            "GET /path HTTP/1.1\r\n\r\n".to_owned(),
            "HTTP/1.1 400 Bad Request",
            None,
            Outcome::Malformed,
        ),
        (
            "garbage\r\n\r\n".to_owned(),
            "HTTP/1.1 400 Bad Request",
            None,
            Outcome::Malformed,
        ),
    ];
    let mut want_outcomes = Vec::new();
    for (request, status, names, outcome) in rows {
        let (mut near, far) = UnixStream::pair().expect("pair");
        timed(&near);
        let gate = rig.gate.clone();
        let worker = std::thread::spawn(move || gate.serve(far));
        near.write_all(request.as_bytes()).expect("write");
        let mut reply = String::new();
        near.read_to_string(&mut reply).expect("reply");
        worker.join().expect("serve ends");
        let first = request.lines().next().unwrap_or_default();
        assert!(reply.starts_with(status), "{first:?}: {reply:?}");
        let (_, body) = reply.split_once("\r\n\r\n").expect("body");
        if let Some(name) = names {
            assert!(body.contains(name), "{first:?}: {body:?}");
        }
        want_outcomes.push(outcome);
    }
    let got: Vec<Outcome> = rig
        .journal
        .0
        .lock()
        .expect("lock")
        .iter()
        .map(|e| e.outcome)
        .collect();
    assert_eq!(got, want_outcomes);
}
