//! `inferd.toml` and the directories: where the catalog, the sockets, the audit file and the
//! configuration are.
//!
//! The engine programs and the caller table come from the file, never from the environment (the one
//! exception is llama.cpp's server at a fixed, root-held system path, see [`ProgramSearch`]); the
//! directories come from the XDG variables (injected, so a test names its own).

mod discover;

use crate::probe::ProbeConfig;
use crate::structured::AiConfig;
use base_xdg::{PathError, Xdg};
use inference_router::catalog::CatalogDirs;
use inference_router::local::EngineConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use discover::ProgramSearch;

/// Why the configuration could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    /// The file is not valid TOML of the expected shape.
    #[error("{0}")]
    Toml(#[source] toml::de::Error),
    /// An `[engines.attached."<id>"]` table the checks of the file refuse.
    #[error("{0}")]
    Attached(#[from] crate::attached::AttachedError),
    /// Neither `$XDG_RUNTIME_DIR` nor a home to place the sockets in.
    #[error("no $XDG_RUNTIME_DIR and no $HOME")]
    NoDirs,
}

/// The file's contents. Every table is optional; a file that does not exist is the default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferdConfig {
    /// Engine programs and the weights cache.
    #[serde(default)]
    pub engines: EngineConfig,
    /// The `ai.*` settings rows, at their paths.
    #[serde(default)]
    pub ai: AiConfig,
    /// Which ports are probed for the runtimes the person runs themselves (Ollama, llama.cpp,
    /// LM Studio), and how often. Left out of the file while it is the default.
    #[serde(default, skip_serializing_if = "ProbeConfig::is_default")]
    pub probe: ProbeConfig,
}

impl InferdConfig {
    /// The configuration in `text`. An attached engine that is named wrongly (no `where`, a
    /// `url` that is not loopback, both a socket and a url) refuses the whole file.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] when the text is not a configuration or an attached engine is named wrongly.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(text).map_err(ConfigError::Toml)?;
        config.engines.attached()?;
        Ok(config)
    }
}

/// Where everything is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    /// `$XDG_CONFIG_HOME/bardo/inference/inferd.toml`: inferd's own folder, where Settings looks for it.
    pub config: PathBuf,
    /// The catalog directories.
    pub catalog: CatalogDirs,
    /// `$XDG_RUNTIME_DIR/bardo/inference`: the engines' sockets.
    pub sockets: PathBuf,
    /// `$XDG_STATE_HOME/bardo/inference/audit.jsonl`.
    pub audit: PathBuf,
    /// `$XDG_STATE_HOME/bardo/inference/spend.json`: what hosted models have cost this day and month.
    pub spend: PathBuf,
    /// `$XDG_STATE_HOME/bardo/inference/tool-proofs.json`: which models have shown they can make a
    /// valid tool call (model ids and a word, never a prompt).
    pub tool_proofs: PathBuf,
    /// `$XDG_STATE_HOME/bardo/inference/computers.toml`: the computers Settings added. inferd's own
    /// file; the hand-written `inferd.toml` is never written.
    pub computers: PathBuf,
    /// `$XDG_STATE_HOME/bardo/inference/computer-keys`: the keys of those computers, one file each,
    /// readable by their owner alone.
    pub computer_keys: PathBuf,
    /// `$XDG_STATE_HOME/bardo/inference/guests.toml`: the answers the person gave about the
    /// computers that ask to use this one's models.
    pub guests: PathBuf,
    /// `$HF_HOME/hub`, else `~/.cache/huggingface/hub`: the weights, when the file names none.
    pub hf_cache: PathBuf,
    /// `bardo/accounts/providers` under the first data directory that has it (`/usr/local/share` or
    /// `/usr/share` by default): the provider files this install ships.
    pub providers_system: PathBuf,
    /// `$XDG_DATA_HOME/bardo/accounts/providers`: the person's own provider files.
    pub providers_own: PathBuf,
    /// `$XDG_DATA_HOME/bardo/engines`: the engines installed for this person alone.
    pub engines_own: PathBuf,
    /// `bardo/accounts/clients.toml` under the first data directory that has it.
    pub clients_shipped: PathBuf,
    /// `$XDG_CONFIG_HOME/bardo/accounts/clients.toml`.
    pub clients_own: PathBuf,
    /// The `applications` directories that hold desktop entries, the person's first: how an app
    /// is named on a consent sheet.
    pub applications: Vec<PathBuf>,
}

impl Dirs {
    /// The directories for the variables `var` answers (`HOME`, `XDG_CONFIG_HOME`,
    /// `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_RUNTIME_DIR`, `HF_HOME`).
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] when a variable the directories need is missing.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let xdg = Xdg::new(var);
        let dir =
            |name: &str, fallback: &str| xdg.dir(name, fallback).ok().ok_or(ConfigError::NoDirs);
        let config = dir("XDG_CONFIG_HOME", ".config")?;
        let data = dir("XDG_DATA_HOME", ".local/share")?;
        let state = dir("XDG_STATE_HOME", ".local/state")?;
        let runtime = xdg.var_dir("XDG_RUNTIME_DIR").ok_or(ConfigError::NoDirs)?;
        let hf_cache = xdg
            .raw("HF_HOME")
            .map(|hf| PathBuf::from(hf).join("hub"))
            .or_else(|| xdg.home().map(|home| home.join(".cache/huggingface/hub")))
            .ok_or(ConfigError::NoDirs)?;
        // The `applications` directories that name an app for a person (the desktop entries):
        // the person's first, then each absolute `$XDG_DATA_DIRS` entry's.
        let applications = std::iter::once(data.clone())
            .chain(xdg.data_dirs())
            .map(|dir| dir.join("applications"))
            .collect();
        Ok(Self {
            providers_system: xdg.shipped("bardo/accounts/providers"),
            providers_own: data.join("bardo").join("accounts").join("providers"),
            engines_own: data.join("bardo").join("engines"),
            clients_shipped: xdg.shipped("bardo/accounts/clients.toml"),
            clients_own: config.join("bardo").join("accounts").join("clients.toml"),
            applications,
            config: config.join("bardo").join("inference").join("inferd.toml"),
            catalog: CatalogDirs {
                system: xdg.shipped("bardo/models/catalog"),
                user: data.join("bardo").join("models").join("catalog"),
            },
            sockets: runtime.join("bardo").join("inference"),
            audit: state.join("bardo").join("inference").join("audit.jsonl"),
            spend: state.join("bardo").join("inference").join("spend.json"),
            tool_proofs: state
                .join("bardo")
                .join("inference")
                .join("tool-proofs.json"),
            computers: state.join("bardo").join("inference").join("computers.toml"),
            computer_keys: state.join("bardo").join("inference").join("computer-keys"),
            guests: state.join("bardo").join("inference").join("guests.toml"),
            hf_cache,
        })
    }

    /// The directories of the process's environment.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] when a variable the directories need is missing.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_vars(|name| std::env::var(name).ok())
    }
}

impl InferdConfig {
    /// The engines' weights cache: the file's, or the default; and the programs found at the
    /// system's fixed paths ([`ProgramSearch::system`]).
    #[must_use]
    pub fn engines_in(&self, dirs: &Dirs) -> EngineConfig {
        self.engines_with(dirs, &ProgramSearch::system())
    }

    /// [`Self::engines_in`] with the fixed paths looked in by `programs`. The one place the
    /// effective engine programs are decided: the engine starter, the picker's note and
    /// `LocalEngines` all read what this returns.
    #[must_use]
    pub fn engines_with(&self, dirs: &Dirs, programs: &ProgramSearch) -> EngineConfig {
        let mut engines = self.engines.clone();
        engines.llama_server = programs.llama_server(engines.llama_server.as_deref());
        engines.vllm_python = programs.vllm_python(engines.vllm_python.as_deref());
        if engines.hf_cache.as_os_str().is_empty() {
            engines.hf_cache.clone_from(&dirs.hf_cache);
        }
        engines
    }
}

#[cfg(test)]
mod tests;
