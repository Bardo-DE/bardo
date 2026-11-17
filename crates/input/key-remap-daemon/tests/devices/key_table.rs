//! The harness names keys itself; this is the one place that checks it against the key remap daemon's table.
//! It touches no device and runs in a plain `cargo test`.

use key_remap_daemon::Key;
use key_remap_devtest::KEYS;

#[test]
fn every_harness_key_has_the_code_key_remap_gives_its_name() {
    for &(name, code) in KEYS {
        let key = Key::named(name).unwrap_or_else(|| panic!("key_remap has no key named {name:?}"));
        assert_eq!(key.code(), code, "{name}");
        assert_eq!(key.name(), name, "code {code}");
    }
}
