//! `bardo-sandbox-forward run --listen 127.0.0.1:<port> --socket <path> -- <program> [args...]`
//!
//! See [`sandbox::cli::forward_main`].

fn main() -> std::process::ExitCode {
    sandbox::cli::forward_main("bardo-sandbox-forward")
}
