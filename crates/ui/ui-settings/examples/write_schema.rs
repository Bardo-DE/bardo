//! `cargo run -p ui-settings --example write_schema -- --write-schema <dir>`: writes
//! `bardo.settings.toml`, the schema of `bardo/appearance.toml`, the way `bardo-shell --write-schema`
//! writes bardo-shell's (design/22-SETTINGS.md section 9.2). Bardo has no app binary, so this example
//! is its `--write-schema`.

use std::process::ExitCode;

use settings_store_fs::maybe_write_schema;
use ui_settings::bardo_schema;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match maybe_write_schema(&bardo_schema(), &args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => {
            eprintln!("usage: write_schema --write-schema <dir>");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("write_schema: {error}");
            ExitCode::FAILURE
        }
    }
}
