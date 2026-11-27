//! `cargo run -p appearance-settings --example doc_tables -- [--schema <file>]... <doc.md>`:
//! rewrites the key tables of design/22-SETTINGS.md (the ones between `keys:begin <domain>` and
//! `keys:end <domain>` comment lines) from the UI kit's own schema and any `<app>.settings.toml` named
//! with `--schema`. The `doc_tables` test of this crate fails when the doc differs from the schema.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use appearance_settings::bardo_schema;
use settings_types::schema::{Schema, regenerate};

/// What the command line asked for.
struct Request {
    doc: PathBuf,
    schemas: Vec<PathBuf>,
}

fn request(args: &[String]) -> Option<Request> {
    let mut doc = None;
    let mut schemas = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--schema" => schemas.push(PathBuf::from(args.next()?)),
            other => doc = Some(PathBuf::from(other)),
        }
    }
    Some(Request { doc: doc?, schemas })
}

fn load(path: &Path) -> Result<Schema, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Schema::from_toml(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn run(request: &Request) -> Result<(), String> {
    let mut schemas = vec![bardo_schema()];
    for path in &request.schemas {
        schemas.push(load(path)?);
    }
    let old = std::fs::read_to_string(&request.doc)
        .map_err(|e| format!("{}: {e}", request.doc.display()))?;
    let new = regenerate(&old, &schemas).map_err(|e| format!("{}: {e}", request.doc.display()))?;
    if new == old {
        return Ok(());
    }
    std::fs::write(&request.doc, new).map_err(|e| format!("{}: {e}", request.doc.display()))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(request) = request(&args) else {
        eprintln!("usage: doc_tables [--schema <file>]... <doc.md>");
        return ExitCode::FAILURE;
    };
    match run(&request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("doc_tables: {error}");
            ExitCode::FAILURE
        }
    }
}
