//! `serving` against a private bus: never the session bus.

use base_testkit::PrivateBus;

struct Echo;

#[zbus::interface(name = "org.example.Echo1")]
impl Echo {
    #[expect(
        clippy::unused_self,
        reason = "a zbus interface method takes `&self` whether or not it reads it"
    )]
    fn echo(&self, text: String) -> String {
        text
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_name_claimed_after_serving_answers_its_first_call() {
    let bus = PrivateBus::start();
    for _ in 0..20 {
        let server = bus.connect().await;
        let client = bus.connect().await;
        server
            .object_server()
            .at("/org/example/Echo", Echo)
            .await
            .expect("registered");
        dbus_kit::ready::serving(&server)
            .await
            .expect("the server answers");
        server
            .request_name("org.example.Echo")
            .await
            .expect("named");
        let reply = client
            .call_method(
                Some("org.example.Echo"),
                "/org/example/Echo",
                Some("org.example.Echo1"),
                "Echo",
                &("hi",),
            )
            .await
            .expect("the first call is answered");
        let text: String = reply.body().deserialize().expect("a string");
        assert_eq!(text, "hi");
        server
            .release_name("org.example.Echo")
            .await
            .expect("released");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_connection_with_no_objects_still_comes_ready() {
    let bus = PrivateBus::start();
    let connection = bus.connect().await;
    dbus_kit::ready::serving(&connection)
        .await
        .expect("the root node answers Peer");
}
