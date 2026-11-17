# Findings

Open items and standing facts. An entry names the condition that closes it.

## The cut

| Where | Closes when |
| --- | --- |
| The root `Cargo.toml` names the accounts service by `rev = "ACCOUNT_REV"` | the gate session fills in the accounts rev that has `account_wire::names::{INFERENCE_BUS, INFERENCE_PATH}` and no `inference` group in account-dbus, and relocks |
| `inference-dbus` keeps a private copy of three small helpers of account-dbus's `json_value` (`bad`, `into_object`, `into_string`, `pub(crate)` there) | account-dbus exports them, or the need codec stops using them |
| `#[bus_interface]` expands to `crate::Call` and `crate::Unserved`, so `inference-dbus`'s `lib.rs` has private `use account_dbus::{Call, Unserved}` for it | the macro names its paths through a crate-root argument or `account_dbus::` |
| The cut compiles nowhere yet: no `cargo` was run in this lane | the gate session's first green `scripts/check-boundary.sh`, clippy and tests |
| `.github/workflows/ci.yml` and `portable.yml` | the repository secret `DEPS_TOKEN` (read access to the accounts service, the model layer and Bardo) exists and the first run of each is green; `bus-tests` is `continue-on-error` until it has passed on a hosted runner |
| `scripts/check-boundary.sh` has no HIDDEN-crates rows (who may depend on `account-dbus`, `account-callers`, `account-settings`, `account-store`) | they are decided with the accounts service's side of the cut; today the EDGES rows pin every direct edge of the nine crates |
| The test `every_app_inferds_table_names_is_the_same_app_in_accountds` (Q10: inferd's and accountd's shipped caller tables name the same app and programs per unit) was dropped from `crates/inferd/tests/it/dist.rs`: it read the accounts service's `dist/callers.toml`, which is not in this repository | a check that both tables agree runs where both files are present (the accounts service's gate with an inference checkout, or a CI step that fetches the accounts service's table at the pinned rev) |
| The paths `share/bardo/inference/`, `share/doc/bardo/` and `/etc/bardo/inferd-callers.toml` | renamed with their readers (2026-11-16) |

## Facts carried over from the accounts service

- inferd probes `127.0.0.1` only, at the ports of `[probe]`: Ollama `11434`, llama.cpp `8080`,
  LM Studio `1234`. A LAN host is never auto-probed, by design.
- A stopped runtime is seen at the next look (at most `[probe] longest_s`, 120 s by default; `Rescan`
  looks at once).
- Probed models: licence is `open("unknown")`, no `StructuredOutput` feature, the context is what the
  runtime reports or the 2048 floor.
