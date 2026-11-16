use engine_supervisor::{Attempt, EngineFailure, EngineState, ExitCode, MonoMs};
use model_catalog::MiB;

fn round_trip<T>(value: &T, json: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + core::fmt::Debug,
{
    assert_eq!(serde_json::to_string(value).unwrap(), json);
    assert_eq!(&serde_json::from_str::<T>(json).unwrap(), value);
}

#[test]
fn states_round_trip_with_pinned_json() {
    round_trip(&EngineState::Stopped, r#"{"kind":"stopped"}"#);
    round_trip(
        &EngineState::Starting {
            since: MonoMs(10),
            attempt: Attempt(1),
        },
        r#"{"kind":"starting","v":{"since":10,"attempt":1}}"#,
    );
    round_trip(
        &EngineState::Ready {
            since: MonoMs(20),
            last_used: MonoMs(30),
        },
        r#"{"kind":"ready","v":{"since":20,"last_used":30}}"#,
    );
    round_trip(
        &EngineState::Failed(EngineFailure::NoRoom {
            need: MiB(12_000),
            free: MiB(4_000),
        }),
        r#"{"kind":"failed","v":{"kind":"no_room","v":{"need":12000,"free":4000}}}"#,
    );
    round_trip(
        &EngineState::Failed(EngineFailure::Exited {
            code: ExitCode(137),
        }),
        r#"{"kind":"failed","v":{"kind":"exited","v":{"code":137}}}"#,
    );
}
