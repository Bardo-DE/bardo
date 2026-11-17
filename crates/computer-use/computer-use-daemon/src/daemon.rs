//! Starting the daemon: what the process was given (its arguments and its environment) arrives
//! here as values, and the binary's `main` only passes them in.

use crate::{
    Apps, ComputerUseDaemonConfig, ComputerUseSettings, ServeError, Supervisor, SystemClock,
    caller_identity, follow_settings, locator, read_callers, serve_on_as, watch_settings,
};
use computer_use_host::{RULES_FILE, parse_rules, shipped_rules};
use computer_use_run::Rules;
use computer_use_system::{Links, ReachError, Reached, SystemSeams};
use futures_util::StreamExt;
use settings_store_fs::WatchState;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};

/// Why the daemon did not start or stopped badly.
#[derive(Debug, thiserror::Error)]
pub enum FatalError {
    /// The configuration, the schema directory or the callers table could not be used.
    #[error("{0}")]
    Config(String),
    /// The session bus could not be reached.
    #[error("session bus: {0}")]
    Bus(String),
    /// The compositor the configuration names could not be reached.
    #[error("compositor: {0}")]
    Compositor(#[from] ReachError),
    /// The bus names could not be taken.
    #[error("{0}")]
    Serve(#[from] ServeError),
    /// The signal handler could not be installed.
    #[error("signal handler: {0}")]
    Signal(String),
}

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    /// `--write-schema DIR`: write `computer-use.settings.toml` there for a local install (design/22
    /// section 9.2) and stop.
    WriteSchema(PathBuf),
    /// Serve, with the configuration file named by `--config` or `$BARDO_COMPUTER_USE_CONFIG`, if any.
    Serve(Option<PathBuf>),
}

/// The value after `flag` in `args`, if the flag is there.
fn after<'a>(args: &'a [String], flag: &str) -> Option<&'a String> {
    args.iter().skip_while(|arg| *arg != flag).nth(1)
}

fn command(args: &[String], env: &impl Fn(&str) -> Option<String>) -> Command {
    if let Some(dir) = after(args, "--write-schema") {
        return Command::WriteSchema(PathBuf::from(dir));
    }
    let named = after(args, "--config").cloned();
    Command::Serve(
        named
            .or_else(|| env("BARDO_COMPUTER_USE_CONFIG"))
            .map(PathBuf::from),
    )
}

fn write_schema(dir: &Path) -> Result<(), FatalError> {
    let io = |e: std::io::Error| FatalError::Config(format!("{}: {e}", dir.display()));
    std::fs::create_dir_all(dir).map_err(io)?;
    std::fs::write(dir.join("computer-use.settings.toml"), crate::SCHEMA).map_err(io)
}

fn load_config(path: Option<&Path>) -> Result<ComputerUseDaemonConfig, FatalError> {
    let Some(path) = path else {
        return Ok(ComputerUseDaemonConfig::default());
    };
    match std::fs::read_to_string(path) {
        Ok(text) => {
            ComputerUseDaemonConfig::from_toml(&text).map_err(|e| FatalError::Config(e.to_string()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(ComputerUseDaemonConfig::default())
        }
        Err(e) => Err(FatalError::Config(format!("{}: {e}", path.display()))),
    }
}

/// The screen rules: the administrator's `screen-rules.toml` (in the folder of the administrator's
/// files, `/etc/bardo`) when there is one, else the file the program ships.
fn load_rules(env: &impl Fn(&str) -> Option<String>) -> Result<Rules, FatalError> {
    let path = crate::callers::system_dir(env).join(RULES_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            parse_rules(&text).map_err(|e| FatalError::Config(format!("{}: {e}", path.display())))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(shipped_rules()),
        Err(e) => Err(FatalError::Config(format!("{}: {e}", path.display()))),
    }
}

/// Runs the daemon for the command line `args` (without the program name), reading the
/// environment through `env`.
///
/// It connects to the session bus, tries the compositor (no compositor is not a failure: the
/// backend reads `none`), serves `org.bardo.ComputerUse1` and `org.bardo.ComputerUse`, and
/// returns when the bus goes away or it is sent SIGTERM.
///
/// # Errors
///
/// Returns [`Fatal`] when the configuration, rules or callers table cannot be used, the session
/// bus or a named compositor cannot be reached, the bus names cannot be taken, or the SIGTERM
/// handler cannot be installed.
pub async fn run(
    args: &[String],
    env: impl Fn(&str) -> Option<String> + Copy,
) -> Result<(), FatalError> {
    let config_path = match command(args, &env) {
        Command::WriteSchema(dir) => return write_schema(&dir),
        Command::Serve(path) => path,
    };
    let config = load_config(config_path.as_deref())?;
    let rules = load_rules(&env)?;
    // The desktop's shortcut conventions, read once and told to every run's model.
    let platform = keymap::detect(keymap::HostOs::compiled(), env);
    let (keymap, problems) = crate::desktop_keymap(platform, env);
    for problem in problems {
        eprintln!("bardo-computer-use: {problem}");
    }
    let hints: Vec<String> = computer_use_run::shortcut_hint(&keymap)
        .into_iter()
        .collect();
    let connection = zbus::Connection::session()
        .await
        .map_err(|e| FatalError::Bus(e.to_string()))?;
    let reached = Reached::find(&config, &connection).await?;
    eprintln!("bardo-computer-use: {}", reached.describe());
    let links = Links::over(&connection, config.tier);
    let parts = SystemSeams::parts(&config, links, reached, SystemClock::start(), rules);
    let supervisor = Arc::new(Supervisor::with_hints(parts, config, hints));
    // The person's settings (`computer-use/settings.toml`) over the configuration's values, now and
    // whenever the file changes. Until the file lists an app, none is on.
    let base = ComputerUseSettings {
        config: supervisor.settings(),
        apps: Apps::Listed(Vec::new()),
    };
    let mut watched = watch_settings(&locator(&env), base);
    if let WatchState::Blind { reason } = watched.state() {
        eprintln!("bardo-computer-use: settings are read once, not watched: {reason}");
    }
    supervisor.adopt(&watched.current());
    let following = Arc::clone(&supervisor);
    let announcing = connection.clone();
    tokio::spawn(async move { follow_settings(&announcing, &following, &mut watched).await });
    // Who is calling is the process behind the connection, named by the callers table: the
    // system's `computer-use-callers.toml` with the person's laid over it.
    let table = read_callers(&env).map_err(|e| FatalError::Config(e.to_string()))?;
    let who = caller_identity(connection.clone(), table, &env);
    // SIGTERM is caught before the names are taken: whoever sees the name may stop the daemon.
    let mut terminate =
        signal(SignalKind::terminate()).map_err(|e| FatalError::Signal(e.to_string()))?;
    let _serving = serve_on_as(&connection, supervisor, who).await?;

    let mut messages = zbus::MessageStream::from(&connection);
    tokio::select! {
        () = async { while messages.next().await.is_some() {} } => {}
        _ = terminate.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| (*a).to_owned()).collect()
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn the_command_line_wins_over_the_environment() {
        let env = |key: &str| (key == "BARDO_COMPUTER_USE_CONFIG").then(|| "/env.toml".to_owned());
        assert_eq!(
            command(&args(&["--config", "/arg.toml"]), &env),
            Command::Serve(Some(PathBuf::from("/arg.toml")))
        );
        assert_eq!(
            command(&args(&[]), &env),
            Command::Serve(Some(PathBuf::from("/env.toml")))
        );
        assert_eq!(command(&args(&[]), &no_env), Command::Serve(None));
    }

    #[test]
    fn writing_the_schema_takes_the_directory_after_its_flag() {
        assert_eq!(
            command(
                &args(&["--config", "/c.toml", "--write-schema", "/out"]),
                &no_env
            ),
            Command::WriteSchema(PathBuf::from("/out"))
        );
    }
}
