//! Shared builders: the task set on disk, and a way to run a future that never waits on a clock.

use computer_use_eval::corpus::Corpus;
use computer_use_eval::eval::run_scripted;
use computer_use_eval::report::Check;
use computer_use_eval::run::RunRecord;
use std::path::PathBuf;
use std::sync::OnceLock;

/// The task set under `eval/` at the root of the workspace.
pub fn eval_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../eval")
}

pub fn corpus() -> Corpus {
    Corpus::load(&eval_dir()).expect("the task set loads")
}

/// Every script of every task, played once for the whole test binary: the full set on
/// software-rendered surfaces is most of the binary's time.
pub fn all_scripted() -> &'static (Vec<RunRecord>, Vec<Check>) {
    static RUNS: OnceLock<(Vec<RunRecord>, Vec<Check>)> = OnceLock::new();
    RUNS.get_or_init(|| block_on(run_scripted(&corpus(), &[])).expect("the scripts play"))
}

/// Runs `future` on a runtime of its own.
pub fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(future)
}
