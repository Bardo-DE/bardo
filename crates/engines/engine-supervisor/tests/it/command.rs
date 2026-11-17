//! `command`: the unit each engine kind gets, from the shipped catalog entries.

use std::path::PathBuf;

use base_ids::Tokens;
use engine_spec::{EnginePaths, ProgramPath};
use engine_supervisor::{
    EnvPair, GpuAccess, MissingProgramError, Network, Sandbox, SocketPath, UnitSpec, Window,
};
use model_catalog::{
    ContextArg, EngineArg, EngineKind, EngineProfile, FileName, MiB, ModelEntry, WeightFiles,
    parse_entry,
};

const HOLO: &str = include_str!("../../../../models/catalog/holo-3.1-4b.toml");
const KOKORO: &str = include_str!("../../../../models/catalog/kokoro-82m.toml");
const NEMOTRON: &str = include_str!("../../../../models/catalog/nemotron-3.5-asr-streaming.toml");

fn paths() -> EnginePaths {
    let program = |kind, path: &str| (kind, ProgramPath(PathBuf::from(path)));
    EnginePaths {
        programs: [
            program(EngineKind::Vllm, "/opt/vllm/bin/python"),
            program(EngineKind::LlamaServer, "/opt/llama/bin/llama-server"),
            program(EngineKind::SpeechHost, "/opt/bardo/bin/bardo-speech-host"),
            program(EngineKind::KokoroFastApi, "/opt/kokoro/bin/python"),
            program(EngineKind::Strata, "/opt/strata/.venv/bin/python"),
        ]
        .into(),
        hf_cache: PathBuf::from("/data/hf/hub"),
    }
}

fn command(
    entry: &ModelEntry,
    profile: &EngineProfile,
    paths: &EnginePaths,
    socket: &SocketPath,
) -> UnitSpec {
    engine_supervisor::command(entry, profile, paths, socket).unwrap()
}

fn command_with_window(
    entry: &ModelEntry,
    profile: &EngineProfile,
    paths: &EnginePaths,
    socket: &SocketPath,
    window: Window,
) -> UnitSpec {
    engine_supervisor::command_with_window(entry, profile, paths, socket, window).unwrap()
}

#[test]
fn an_engine_with_no_listed_program_cannot_start() {
    let holo = entry(HOLO);
    let none = EnginePaths::new(PathBuf::from("/data/hf/hub"));
    assert_eq!(
        engine_supervisor::command(&holo, &holo.engines[0], &none, &socket("x")),
        Err(MissingProgramError {
            kind: EngineKind::Vllm
        })
    );
}

fn socket(name: &str) -> SocketPath {
    SocketPath(PathBuf::from(format!(
        "/run/user/1000/bardo/inference/{name}.sock"
    )))
}

fn args(unit: &UnitSpec) -> Vec<&str> {
    unit.args.iter().map(|a| a.0.as_str()).collect()
}

fn entry(text: &str) -> ModelEntry {
    parse_entry(text).unwrap()
}

const HOLO_DIR: &str =
    "/data/hf/hub/models--Hcompany--Holo-3.1-4B/snapshots/8c88265a5a159bfd1492db9243733dd2e6e04a6e";

#[test]
fn holo_under_vllm_is_the_pinned_unit() {
    let holo = entry(HOLO);
    let unit = command(&holo, &holo.engines[0], &paths(), &socket("vllm-holo"));
    assert_eq!(
        unit,
        UnitSpec {
            program: ProgramPath(PathBuf::from("/opt/vllm/bin/python")),
            args: [
                "-m",
                "vllm.entrypoints.openai.api_server",
                "--model",
                HOLO_DIR,
                "--served-model-name",
                "holo-3.1-4b",
                "--uds",
                "/run/user/1000/bardo/inference/vllm-holo.sock",
                "--limit-mm-per-prompt",
                "{\"image\":3,\"video\":0}",
                "--enable-auto-tool-choice",
                "--tool-call-parser",
                "qwen3_coder",
                "--reasoning-parser",
                "qwen3",
                "--gpu-memory-utilization",
                "0.80",
                "--max-model-len",
                "32768",
            ]
            .map(|a| EngineArg(a.into()))
            .to_vec(),
            env: vec![EnvPair {
                name: "HF_HUB_OFFLINE".into(),
                value: "1".into()
            }],
            sandbox: Sandbox {
                network: Network::None,
                read: vec![PathBuf::from("/data/hf/hub/models--Hcompany--Holo-3.1-4B")],
                write: vec![PathBuf::from("/run/user/1000/bardo/inference")],
                gpu: GpuAccess::Nvidia,
                memory_max: MiB(4096 + 10400 + 1500),
            },
        }
    );
}

/// One engine's unit: its program, its argv pinned, and what it asks of the machine.
struct Pinned {
    what: &'static str,
    entry: ModelEntry,
    profile: EngineProfile,
    socket: &'static str,
    program: &'static str,
    args: Vec<String>,
    gpu: Option<GpuAccess>,
    memory_max: Option<MiB>,
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one table of engines with the unit each must give: a flat list of expectations"
)]
fn each_engine_gets_its_program_and_its_pinned_argv() {
    let holo = entry(HOLO);
    let nemotron = entry(NEMOTRON);
    let kokoro = entry(KOKORO);
    let llama_profile = EngineProfile {
        kind: EngineKind::LlamaServer,
        context_arg: None,
        inputs: None,
        outputs: None,
        args: vec![
            EngineArg("--jinja".into()),
            EngineArg("--ctx-size".into()),
            EngineArg("32768".into()),
            EngineArg("--special".into()),
        ],
        weights: WeightFiles::Gguf {
            model: FileName("holo-q4_k_m.gguf".into()),
            mmproj: Some(FileName("mmproj-f16.gguf".into())),
        },
    };
    let strata_profile = EngineProfile {
        kind: EngineKind::Strata,
        context_arg: None,
        inputs: None,
        outputs: None,
        args: vec![
            EngineArg("serve/server.py".into()),
            EngineArg("--socket={socket}".into()),
        ],
        weights: WeightFiles::HfSnapshot,
    };
    let strings = |list: &[&str]| list.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>();
    let rows = [
        Pinned {
            what: "llama-server gets the gguf files and the socket as its host",
            entry: holo.clone(),
            profile: llama_profile,
            socket: "llama-holo",
            program: "/opt/llama/bin/llama-server",
            args: vec![
                "--model".into(),
                format!("{HOLO_DIR}/holo-q4_k_m.gguf"),
                "--mmproj".into(),
                format!("{HOLO_DIR}/mmproj-f16.gguf"),
                "--alias".into(),
                "holo-3.1-4b".into(),
                "--host".into(),
                "/run/user/1000/bardo/inference/llama-holo.sock".into(),
                "--jinja".into(),
                "--ctx-size".into(),
                "32768".into(),
                "--special".into(),
            ],
            gpu: Some(GpuAccess::Nvidia),
            memory_max: None,
        },
        Pinned {
            what: "the speech host runs on the cpu with the weights directory and the socket",
            profile: nemotron.engines[0].clone(),
            entry: nemotron,
            socket: "speech-in",
            program: "/opt/bardo/bin/bardo-speech-host",
            args: strings(&[
                "--model-dir",
                "/data/hf/hub/models--csukuangfj2--sherpa-onnx-nemotron-3.5-asr-streaming-0.6b-560ms-int8-2026-06-11/snapshots/ab43d895f5985b1bbab8b6eac8607fcdc05343f3",
                "--socket",
                "/run/user/1000/bardo/inference/speech-in.sock",
                "--threads",
                "6",
                "--chunk-ms",
                "560",
            ]),
            // An all-zero estimate means no GPU, and a CPU entry still has a host-memory cap.
            gpu: Some(GpuAccess::Absent),
            memory_max: Some(MiB(4096)),
        },
        Pinned {
            what: "kokoro runs uvicorn in its own environment on the socket",
            profile: kokoro.engines[0].clone(),
            entry: kokoro,
            socket: "kokoro",
            program: "/opt/kokoro/bin/python",
            args: strings(&[
                "-m",
                "uvicorn",
                "api.src.main:app",
                "--uds",
                "/run/user/1000/bardo/inference/kokoro.sock",
            ]),
            gpu: Some(GpuAccess::Absent),
            memory_max: None,
        },
        Pinned {
            what: "strata runs its own python with the profile args only",
            entry: holo,
            profile: strata_profile,
            socket: "strata",
            program: "/opt/strata/.venv/bin/python",
            args: strings(&[
                "serve/server.py",
                "--socket=/run/user/1000/bardo/inference/strata.sock",
            ]),
            gpu: None,
            memory_max: None,
        },
    ];
    for row in rows {
        let unit = command(&row.entry, &row.profile, &paths(), &socket(row.socket));
        assert_eq!(
            unit.program,
            ProgramPath(PathBuf::from(row.program)),
            "{}",
            row.what
        );
        assert_eq!(args(&unit), row.args, "{}", row.what);
        if let Some(gpu) = row.gpu {
            assert_eq!(unit.sandbox.gpu, gpu, "{}", row.what);
        }
        if let Some(memory) = row.memory_max {
            assert_eq!(unit.sandbox.memory_max, memory, "{}", row.what);
        }
    }
}

#[test]
fn the_socket_placeholder_is_replaced_wherever_it_appears() {
    let holo = entry(HOLO);
    let profile = EngineProfile {
        kind: EngineKind::SpeechHost,
        context_arg: None,
        inputs: None,
        outputs: None,
        args: vec![
            EngineArg("--bind=unix:{socket}".into()),
            EngineArg("{socket}:{socket}".into()),
            EngineArg("--plain".into()),
        ],
        weights: WeightFiles::SherpaDir,
    };
    let unit = command(&holo, &profile, &paths(), &socket("x"));
    let s = "/run/user/1000/bardo/inference/x.sock";
    assert!(args(&unit).contains(&format!("--bind=unix:{s}").as_str()));
    assert!(args(&unit).contains(&format!("{s}:{s}").as_str()));
    assert!(args(&unit).iter().all(|a| !a.contains("{socket}")));
}

#[test]
fn every_engine_starts_offline_with_no_network_and_a_writable_socket_directory_only() {
    for (text, which) in [(HOLO, 0), (KOKORO, 0), (NEMOTRON, 0)] {
        let e = entry(text);
        let unit = command(&e, &e.engines[which], &paths(), &socket("any"));
        assert_eq!(unit.env.len(), 1);
        assert_eq!(
            (unit.env[0].name.as_str(), unit.env[0].value.as_str()),
            ("HF_HUB_OFFLINE", "1")
        );
        assert_eq!(unit.sandbox.network, Network::None);
        assert_eq!(
            unit.sandbox.write,
            vec![PathBuf::from("/run/user/1000/bardo/inference")]
        );
        assert_eq!(
            unit.sandbox.read.len(),
            1,
            "the weights directory, read-only"
        );
    }
}

/// No component walks up or is a root: joined to the cache, the path stays under it.
fn stays_inside(path: &std::path::Path) -> bool {
    path.starts_with("/data/hf/hub")
        && path
            .components()
            .all(|c| !matches!(c, std::path::Component::ParentDir))
}

#[test]
fn a_repository_or_revision_cannot_leave_the_cache() {
    let mut holo = entry(HOLO);
    let model_catalog::WeightSource::HuggingFace { repo, revision } = &mut holo.source else {
        panic!("holo is a Hugging Face entry");
    };
    repo.0 = "../../etc/passwd".into();
    revision.0 = "../../../root".into();
    let unit = command(&holo, &holo.engines[0], &paths(), &socket("x"));
    let model_dir = unit.args[3].0.clone();
    assert!(
        model_dir.starts_with("/data/hf/hub/models--"),
        "{model_dir}"
    );
    assert!(stays_inside(&PathBuf::from(&model_dir)), "{model_dir}");
    assert!(unit.sandbox.read[0].starts_with("/data/hf/hub"));
    assert!(stays_inside(&unit.sandbox.read[0]));
}

#[test]
fn a_text_only_gguf_gets_no_projector() {
    let holo = entry(HOLO);
    let weights = |mmproj: Option<&str>| EngineProfile {
        kind: EngineKind::LlamaServer,
        context_arg: None,
        inputs: None,
        outputs: None,
        args: vec![],
        weights: WeightFiles::Gguf {
            model: FileName("nomic.gguf".into()),
            mmproj: mmproj.map(|name| FileName(name.into())),
        },
    };
    // Absent, and the empty name earlier files wrote for it: the same command, no `--mmproj`.
    let none = command(&holo, &weights(None), &paths(), &socket("t"));
    let empty = command(&holo, &weights(Some("")), &paths(), &socket("t"));
    assert_eq!(none, empty);
    assert!(!args(&none).contains(&"--mmproj"));
    assert_eq!(
        args(&none)[..2],
        ["--model", &format!("{HOLO_DIR}/nomic.gguf")]
    );
    assert_eq!(args(&none)[2], "--alias");
}

#[test]
fn a_gguf_file_name_with_a_directory_part_stays_in_the_snapshot() {
    let holo = entry(HOLO);
    let profile = EngineProfile {
        kind: EngineKind::LlamaServer,
        context_arg: None,
        inputs: None,
        outputs: None,
        args: vec![],
        weights: WeightFiles::Gguf {
            model: FileName("../../outside/model.gguf".into()),
            mmproj: Some(FileName("/abs/mmproj.gguf".into())),
        },
    };
    let unit = command(&holo, &profile, &paths(), &socket("x"));
    assert_eq!(unit.args[1].0, format!("{HOLO_DIR}/model.gguf"));
    assert_eq!(unit.args[3].0, format!("{HOLO_DIR}/mmproj.gguf"));
}

#[test]
fn llama_server_with_weights_that_are_not_gguf_gets_no_model_and_no_panic() {
    let holo = entry(HOLO);
    let profile = EngineProfile {
        kind: EngineKind::LlamaServer,
        context_arg: None,
        inputs: None,
        outputs: None,
        args: vec![],
        weights: WeightFiles::HfSnapshot,
    };
    let unit = command(&holo, &profile, &paths(), &socket("x"));
    assert!(!args(&unit).contains(&"--model"));
}

#[test]
fn command_is_pure_and_total_over_arbitrary_text() {
    use proptest::prelude::*;
    let holo = entry(HOLO);
    let mut runner = proptest::test_runner::TestRunner::default();
    runner
        .run(
            &(".{0,40}", ".{0,40}", ".{0,40}", ".{0,40}"),
            |(repo, revision, arg, file)| {
                let mut edited = holo.clone();
                let model_catalog::WeightSource::HuggingFace {
                    repo: repo_id,
                    revision: revision_id,
                } = &mut edited.source
                else {
                    panic!("the test entry is a Hugging Face entry");
                };
                repo_id.0 = repo;
                revision_id.0 = revision;
                let profile = EngineProfile {
                    kind: EngineKind::LlamaServer,
                    context_arg: None,
                    inputs: None,
                    outputs: None,
                    args: vec![EngineArg(arg)],
                    weights: WeightFiles::Gguf {
                        model: FileName(file.clone()),
                        mmproj: Some(FileName(file)),
                    },
                };
                let first = command(&edited, &profile, &paths(), &socket("p"));
                let second = command(&edited, &profile, &paths(), &socket("p"));
                prop_assert_eq!(&first, &second);
                for path in &first.sandbox.read {
                    prop_assert!(stays_inside(path));
                }
                for arg in first
                    .args
                    .iter()
                    .filter(|a| a.0.starts_with("/data/hf/hub"))
                {
                    prop_assert!(stays_inside(std::path::Path::new(&arg.0)), "{}", arg.0);
                }
                Ok(())
            },
        )
        .unwrap();
}

/// The window arguments of a unit: every `--max-model-len` or `--ctx-size` and its value.
fn window_args<'a>(unit: &'a UnitSpec, flag: &str) -> Vec<&'a str> {
    let all = args(unit);
    all.iter()
        .enumerate()
        .filter(|(_, a)| **a == flag)
        .map(|(at, _)| all[at + 1])
        .collect()
}

#[test]
fn the_window_is_the_chosen_one_or_the_entrys_served_one() {
    let holo = entry(HOLO);
    let llama = |context_arg| EngineProfile {
        kind: EngineKind::LlamaServer,
        context_arg,
        inputs: None,
        outputs: None,
        args: vec![EngineArg("--jinja".into())],
        weights: WeightFiles::Gguf {
            model: FileName("holo.gguf".into()),
            mmproj: None,
        },
    };
    let flag = |name: &str| Some(ContextArg::Flag(EngineArg(name.into())));
    let unit = |profile: &EngineProfile, window| {
        command_with_window(&holo, profile, &paths(), &socket("w"), window)
    };
    // (profile, window, the values of the window flag, what the line ends with)
    for (profile, window, flag_name, want) in [
        (
            &holo.engines[0],
            Window::Serve,
            "--max-model-len",
            vec!["32768"],
        ),
        (
            &holo.engines[0],
            Window::Chosen(Tokens(65536)),
            "--max-model-len",
            vec!["65536"],
        ),
        (&llama(flag("-c")), Window::Serve, "-c", vec!["32768"]),
        (
            &llama(flag("-c")),
            Window::Chosen(Tokens(8192)),
            "-c",
            vec!["8192"],
        ),
        // Set per request, or not at all: nothing is passed at start.
        (
            &llama(Some(ContextArg::PerRequest)),
            Window::Chosen(Tokens(8192)),
            "-c",
            vec![],
        ),
        (&llama(None), Window::Chosen(Tokens(8192)), "-c", vec![]),
    ] {
        let unit = unit(profile, window);
        assert_eq!(
            window_args(&unit, flag_name),
            want,
            "{window:?} {flag_name}"
        );
    }
    // `command` is the served window.
    assert_eq!(
        command(&holo, &holo.engines[0], &paths(), &socket("w")),
        command_with_window(
            &holo,
            &holo.engines[0],
            &paths(),
            &socket("w"),
            Window::Serve
        )
    );
    // The window comes last, after the listed args, so it is the one the engine reads.
    let last = unit(&holo.engines[0], Window::Chosen(Tokens(4096)));
    assert_eq!(
        args(&last)[args(&last).len() - 2..],
        ["--max-model-len", "4096"]
    );
}

#[test]
fn an_entry_with_no_window_to_serve_passes_none() {
    let nemotron = entry(NEMOTRON);
    let profile = EngineProfile {
        context_arg: Some(ContextArg::Flag(EngineArg("--max-model-len".into()))),
        ..nemotron.engines[0].clone()
    };
    let unit = command(&nemotron, &profile, &paths(), &socket("s"));
    assert!(window_args(&unit, "--max-model-len").is_empty());
}
