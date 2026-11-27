//! `BARDO_ACCOUNTD_KEYS=file:<absolute path>` on the real binary and a private bus, with no Secret
//! Service anywhere: a `test-keys` build refuses a value or a file it cannot use before it
//! serves, and a shipped build ignores the variable and says so. Nothing here touches the real
//! Secret Service or keyring; the environment is cleared.

use super::accountd::{env_on, serving, spawn};
use base_testkit::PrivateBus;
use std::path::Path;

mod test_build {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &Path) -> u32 {
        std::fs::metadata(path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_test_build_says_where_credentials_are_and_makes_the_key_file_private() {
        let bus = PrivateBus::start();
        let env = env_on(&bus);
        let keys = env.root().join("keys/keys.json");
        let env = env.with("BARDO_ACCOUNTD_KEYS", &format!("file:{}", keys.display()));
        let mut daemon = spawn(&env, &[]);
        assert!(serving(&bus, &mut daemon).await, "{}", daemon.stderr());
        assert!(
            daemon
                .stderr()
                .contains("TEST BUILD: credentials are in the file")
                && daemon.stderr().contains("keys.json"),
            "{}",
            daemon.stderr()
        );
        assert_eq!(mode(&keys), 0o600);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_value_that_is_not_an_absolute_file_path_stops_the_daemon_before_it_serves() {
        for (value, says) in [
            ("memory", "is not file:<absolute path>"),
            ("file:", "is not file:<absolute path>"),
            ("", "is not file:<absolute path>"),
            ("file:relative/keys.json", "is not absolute"),
        ] {
            let bus = PrivateBus::start();
            let env = env_on(&bus).with("BARDO_ACCOUNTD_KEYS", value);
            let mut daemon = spawn(&env, &[]);
            assert!(!serving(&bus, &mut daemon).await, "{value}");
            assert!(!daemon.wait().success(), "{value}");
            assert!(
                daemon.stderr().contains(says),
                "{value}: {}",
                daemon.stderr()
            );
            assert!(!env.root().join("relative").exists());
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_key_file_open_to_group_or_other_stops_the_daemon_and_is_left_alone() {
        let bus = PrivateBus::start();
        let env = env_on(&bus);
        let keys = env.root().join("keys.json");
        std::fs::write(&keys, r#"{"version":1,"items":[]}"#).expect("file");
        std::fs::set_permissions(&keys, std::fs::Permissions::from_mode(0o644)).expect("chmod");
        let env = env.with("BARDO_ACCOUNTD_KEYS", &format!("file:{}", keys.display()));
        let mut daemon = spawn(&env, &[]);
        assert!(!serving(&bus, &mut daemon).await);
        assert!(!daemon.wait().success());
        assert!(daemon.stderr().contains("0600"), "{}", daemon.stderr());
        assert_eq!(mode(&keys), 0o644, "not fixed behind the person's back");
    }
}
