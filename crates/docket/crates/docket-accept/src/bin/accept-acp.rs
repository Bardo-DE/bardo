//! docket-acp as its own process, switched on: the library's `serve` under a name of the
//! acceptance harness (the packaged binary reads the person's settings file; this is the
//! setting `agent.acp.expose = "on"` already read).

use std::process::ExitCode;

fn main() -> ExitCode {
    docket_log::init("docket-acp");
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(why) => {
            eprintln!("docket-acp: no runtime: {why}");
            return ExitCode::FAILURE;
        }
    };
    let env = |key: &str| std::env::var(key).ok();
    let seams = docket_acp_bin::harness::acp::HarnessSeams {
        proc_root: docket_accept::seams::proc_root(&env, docket_accept::seams::ACP_PROC_ROOT_VAR),
        system_callers: docket_accept::seams::system_callers(
            &env,
            docket_accept::seams::ACP_SYSTEM_CALLERS_VAR,
        ),
    };
    match runtime.block_on(docket_acp_bin::harness::acp::serve_with(
        docket_accept::seams::trace(&env),
        seams,
        &env,
    )) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("docket-acp: {why}");
            ExitCode::FAILURE
        }
    }
}
