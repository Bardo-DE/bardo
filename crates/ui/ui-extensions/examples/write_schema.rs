//! `cargo run -p ui-extensions --example write_schema -- --write-schema <dir>`: writes
//! `bardo.extensions.settings.toml`, the schema of `bardo/extensions.toml`, beside the
//! `bardo.settings.toml` that ui-settings' own `write_schema` example writes (design/22-SETTINGS.md
//! section 9.2). Installers run both.

use std::path::PathBuf;
use std::process::ExitCode;

use ui_extensions::extensions_schema;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(dir) = args
        .iter()
        .position(|arg| arg == "--write-schema")
        .and_then(|at| args.get(at + 1))
    else {
        eprintln!("usage: write_schema --write-schema <dir>");
        return ExitCode::FAILURE;
    };
    match extensions_schema().write_part_to(&PathBuf::from(dir), "extensions") {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("write_schema: {error}");
            ExitCode::FAILURE
        }
    }
}
