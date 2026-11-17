//! The providers the launcher panel asks what a chosen row does.
//!
//! `Provider::activate` is pure, but the providers that answer it are `shell-search`'s, which the
//! panel's crate does not name: the daemon builds them over the services' receivers and gives
//! every root `shell_launcher_ui::launcher::LauncherProviders` as a context.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use shell_launcher_ui::launcher::LauncherProviders;
use shell_model::results::LauncherPolicy;
use shell_search::providers::files::IndexSource;
use shell_search::providers::{
    AppsProvider, CalculatorProvider, ClipboardProvider, CompanionProvider, EmojiProvider,
    FilesProvider, MemoryProvider, RunsProvider, SettingsProvider, SystemProvider, ThingsProvider,
    WebProvider, WindowsProvider,
};
use shell_services::env::Buses;
use shell_ui_kit::services::Services;
use tokio::sync::watch;

/// The real providers over `services`' receivers (the first five, then the four whose
/// `activate` reads the row alone: their policy here is the default, and they are never
/// queried, so they reach no bus).
#[must_use]
pub fn launcher_providers(services: &Services) -> LauncherProviders {
    let (_, policy) = watch::channel(LauncherPolicy::default());
    let buses = Buses::absent();
    LauncherProviders(vec![
        Arc::new(AppsProvider {
            apps: services.apps.state.clone(),
            frecency: Arc::new(Mutex::default()),
        }),
        Arc::new(WindowsProvider {
            windows: services.windows.state.clone(),
            apps: services.apps.state.clone(),
        }),
        Arc::new(CalculatorProvider::default()),
        Arc::new(SettingsProvider::default()),
        Arc::new(SystemProvider::new(services.session.state.clone())),
        Arc::new(FilesProvider {
            policy: policy.clone(),
            frecency: Arc::new(Mutex::default()),
            source: IndexSource::Detect(buses.session),
            home: PathBuf::from("/"),
            viewer_store: PathBuf::from("/"),
        }),
        Arc::new(ClipboardProvider {
            history: services.clipboard.state.clone(),
        }),
        Arc::new(EmojiProvider {
            policy: policy.clone(),
            frecency: Arc::new(Mutex::default()),
        }),
        Arc::new(WebProvider { policy }),
        Arc::new(ThingsProvider {
            companion: services.companion.state.clone(),
            control: services.companion.control.clone(),
        }),
        Arc::new(CompanionProvider {
            companion: services.companion.state.clone(),
        }),
        Arc::new(RunsProvider {
            companion: services.companion.state.clone(),
        }),
        Arc::new(MemoryProvider {
            companion: services.companion.state.clone(),
            control: services.companion.control.clone(),
        }),
    ])
}
