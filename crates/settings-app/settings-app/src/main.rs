//! `bardo-settings`: the desktop's Settings app, one window on Blitz.

mod cli;
mod dev_callers;
mod dirs;
mod service;

use clap::Parser;
use cli::Run;
use settings_app_client::{ClientBuilder, ClientHomes};
use settings_app_files::{FirstRunMark, KeymapFiles, discover, find_agents_program};
use settings_app_intents::{
    ServeFault, SettingsProvider, hand_over_session, search_entries, serve_session,
};
use settings_app_link::bus::BusSource;
use settings_app_model::{Catalogue, PageKind, PageSlug, inbox};
use settings_app_system::services;
use settings_app_ui::{App, Env, FirstRunApp, Homes, Opening};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use ui_blitz::{
    AppConfig, AppId, Decorations, FileRoots, LaunchError, WindowSize, enter_runtime, launch,
};
use ui_settings::SystemPrefsSource;

/// The window's size in logical pixels.
const WIDTH: u32 = 900;
const HEIGHT: u32 = 640;

/// The first-run window's.
const FIRST_RUN_WIDTH: u32 = 440;
const FIRST_RUN_HEIGHT: u32 = 520;

/// The window of the assistant's first-run step: short, like the welcome window.
fn window_size(wanted: Option<&PageSlug>) -> WindowSize {
    match wanted {
        Some(slug) if PageKind::is_assistant_setup(slug) => {
            WindowSize::new(FIRST_RUN_WIDTH, FIRST_RUN_HEIGHT)
        }
        Some(_) | None => WindowSize::new(WIDTH, HEIGHT),
    }
}

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    if let Err(code) = dev_callers::take_up(cli.dev_callers()) {
        return code;
    }
    match cli.run() {
        Run::Settings(opening) => settings(opening),
        Run::Setup => first_run(),
        Run::Service => service::run(),
        Run::SearchEntries { file, schemas } => write_search_entries(&file, schemas),
    }
}

/// Writes the launcher's search entries for the pages and keys of the schemas in `schemas` (the
/// data directories' when none are named) to `file`, for the installer. It reads no settings and
/// opens no window, so it needs no home directory.
fn write_search_entries(file: &Path, schemas: Vec<PathBuf>) -> ExitCode {
    let dirs = if schemas.is_empty() {
        dirs::schema_dirs()
    } else {
        schemas
    };
    let found = discover(&dirs);
    for warning in &found.warnings {
        eprintln!("bardo-settings: {warning}");
    }
    let text = search_entries(&Catalogue::build(&found.programs));
    match std::fs::write(file, text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(fault) => {
            eprintln!("bardo-settings: cannot write {}: {fault}", file.display());
            ExitCode::FAILURE
        }
    }
}

/// The environment both windows read, from the person's directories.
fn environment(opening: Opening) -> Result<Env, ExitCode> {
    let (Some(config_home), Some(data_home), Some(state_home), Some(cache_home)) = (
        dirs::config_home(),
        dirs::data_home(),
        dirs::state_home(),
        dirs::cache_home(),
    ) else {
        eprintln!(
            "bardo-settings: neither XDG_CONFIG_HOME nor HOME is set, nowhere to keep settings"
        );
        return Err(ExitCode::FAILURE);
    };
    let path = std::env::var("PATH").unwrap_or_default();
    let real = services::over(
        &BusSource::Session,
        &data_home,
        &cache_home,
        dirs::intent_dirs(),
    );
    let client = ClientBuilder::over(
        real,
        ClientHomes {
            config: config_home.clone(),
            data: data_home.clone(),
        },
    )
    .with_programs_path(path.clone())
    .with_agents_program(find_agents_program(
        std::env::current_exe().ok().as_deref(),
        &path,
    ))
    .build();
    let env = Env::from_schemas(
        discover(&dirs::schema_dirs()),
        Homes {
            config: config_home,
            data: data_home,
            state: state_home,
        },
        SystemPrefsSource::Portal,
        opening,
        client,
    )
    .with_keymap(KeymapFiles::standard())
    .with_application_dirs(dirs::application_dirs())
    .with_system_data_dirs(dirs::system_data_dirs());
    for warning in &env.warnings {
        eprintln!("bardo-settings: {warning}");
    }
    Ok(env)
}

/// The one-time welcome window; a run after it has been shown exits at once, before anything
/// else is read or opened.
fn first_run() -> ExitCode {
    let Some(state_home) = dirs::state_home() else {
        eprintln!(
            "bardo-settings: neither XDG_STATE_HOME nor HOME is set, nowhere to remember the first run"
        );
        return ExitCode::FAILURE;
    };
    if FirstRunMark::under(&state_home).done() {
        return ExitCode::SUCCESS;
    }
    let env = match environment(Opening::default()) {
        Ok(env) => env,
        Err(code) => return code,
    };
    let config = AppConfig::new(
        "Welcome",
        WindowSize::new(FIRST_RUN_WIDTH, FIRST_RUN_HEIGHT),
    )
    .with_app_id(AppId("org.bardo.Settings".to_owned()))
    .with_decorations(Decorations::Client)
    .with_context(env);
    ended(launch(FirstRunApp, config))
}

fn settings(opening: Opening) -> ExitCode {
    let wanted = opening.page.clone();
    let env = match environment(opening) {
        Ok(env) => env,
        Err(code) => return code,
    };
    let (outbox, inbox) = inbox();
    let env = Env { inbox, ..env };
    let config_home = env.config_home.clone();
    // The runtime `launch` will use: the intents service lives on it for the app's life.
    let _runtime = enter_runtime();
    let provider = SettingsProvider::new(
        Arc::clone(&env.catalogue),
        config_home,
        outbox,
        Arc::new(env.client.clone()),
    );
    // Held until the process ends: dropping the connection takes the service off the bus.
    let _service = match provider {
        Ok(provider) => match enter_runtime_block(serve_session(provider)) {
            Ok(connection) => Some(connection),
            Err(ServeFault::Running) => return hand_over_to_running(wanted),
            Err(fault) => {
                eprintln!("bardo-settings: no intents service: {fault}");
                None
            }
        },
        Err(fault) => {
            eprintln!("bardo-settings: no intents service: {fault}");
            None
        }
    };
    // The folders the Wallpaper page draws pictures from; the window reads no other `file:` URL.
    let roots = FileRoots::new(env.client.wallpaper().roots());
    let config = AppConfig::new("Settings", window_size(wanted.as_ref()))
        .with_app_id(AppId("org.bardo.Settings".to_owned()))
        .with_decorations(Decorations::Client)
        .with_file_roots(roots)
        .with_context(env);
    ended(launch(App, config))
}

/// The exit code of a window that ran to its end: failure, with the reason said, when it could not run.
fn ended(launched: Result<(), LaunchError>) -> ExitCode {
    match launched {
        Ok(()) => ExitCode::SUCCESS,
        Err(fault) => {
            eprintln!("bardo-settings: {fault}");
            ExitCode::FAILURE
        }
    }
}

/// Runs `future` to its end on the process-wide runtime, before any window exists.
fn enter_runtime_block<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Handle::current().block_on(future)
}

/// Another Settings is running: ask it to show itself (with the activation token this process
/// was started with) and leave, so there is never a second window. `page` is the page this
/// process was asked to open, which the running Settings shows. The token and the page go only to
/// an owner of the name that holds the Settings role; to any other program nothing is sent and this
/// process fails with the reason (SETTINGS-16).
fn hand_over_to_running(page: Option<PageSlug>) -> ExitCode {
    let token = std::env::var("XDG_ACTIVATION_TOKEN")
        .ok()
        .filter(|token| !token.is_empty());
    let handed = enter_runtime_block(hand_over_session(token, page));
    match handed {
        Ok(()) => ExitCode::SUCCESS,
        Err(fault) => {
            eprintln!(
                "bardo-settings: could not hand over to the Settings that is running: {fault}"
            );
            ExitCode::FAILURE
        }
    }
}
