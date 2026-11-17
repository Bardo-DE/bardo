//! Building a [`Client`]: over the services it is handed (the real ones of the person's session
//! from `settings-app-system`, or what a test hands in), with any of them replaced.

use crate::client::Client;
use crate::picker::{AgentSearch, ReasonFeed};
use settings_app_files::{ConfigHome, DataHome};
use settings_app_link::accounts::AccountsLink;
use settings_app_link::bluetooth::BluetoothLink;
use settings_app_link::computers::ComputersLink;
use settings_app_link::displays::DisplaysLink;
use settings_app_link::engine_install::Desk;
use settings_app_link::engines::EnginesLink;
use settings_app_link::hot_corners::CornerDesk;
use settings_app_link::lending::Lender;
use settings_app_link::memory_consent::MemoryLink;
use settings_app_link::modules::ModulesLink;
use settings_app_link::person::PersonLink;
use settings_app_link::picker::PickerLink;
use settings_app_link::privacy::PrivacyLink;
use settings_app_link::running::RunningLink;
use settings_app_link::screen_sharing::ScreenSharingLink;
use settings_app_link::services::Services;
use settings_app_link::spaces::SpacesLink;
use settings_app_link::sync::SyncLink;
use settings_app_link::updates::{Departure, UpdatesLink};
use settings_app_link::wallpaper::Pictures;
use settings_app_link::wifi::WifiLink;
use std::path::PathBuf;
use std::sync::Arc;

/// The person's own folders the client reads: settings and data (or scratch folders standing in
/// for them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientHomes {
    /// Where the settings files are.
    pub config: ConfigHome,
    /// Where the actions service's agents and the added pictures are.
    pub data: DataHome,
}

/// A client being built: every service is the one it was handed until a setter says otherwise.
#[derive(Debug, Clone)]
pub struct ClientBuilder(Client);

impl ClientBuilder {
    /// A client over `services`, with its files under `homes`.
    #[must_use]
    pub fn over(services: Services, homes: ClientHomes) -> ClientBuilder {
        let ClientHomes { config, data } = homes;
        let Services {
            chooser,
            modules,
            accounts,
            sync,
            picker,
            privacy,
            screen_sharing,
            spaces,
            memory,
            running,
            computers,
            engines,
            lender,
            updates,
            log_out,
            wifi,
            bluetooth,
            displays,
            desk,
            pictures,
            person,
            corners,
        } = services;
        ClientBuilder(Client {
            chooser,
            modules,
            accounts,
            sync,
            picker,
            privacy,
            screen_sharing,
            spaces,
            memory,
            running,
            computers,
            engines,
            reasons: ReasonFeed::default(),
            lender,
            updates,
            log_out,
            wifi,
            bluetooth,
            displays,
            desk,
            pictures,
            person,
            corners,
            search: AgentSearch {
                config_home: config,
                data_home: data,
                programs_path: String::new(),
            },
            agents_program: None,
        })
    }

    /// The client built.
    #[must_use]
    pub fn build(self) -> Client {
        self.0
    }

    /// With the lending switch reaching `lender`.
    #[must_use]
    pub fn with_lender(self, lender: Lender) -> ClientBuilder {
        ClientBuilder(Client { lender, ..self.0 })
    }

    /// With the Software Update pane following `updates`.
    #[must_use]
    pub fn with_updates(self, updates: Arc<dyn UpdatesLink>) -> ClientBuilder {
        ClientBuilder(Client { updates, ..self.0 })
    }

    /// With the Software Update pane logging out through `log_out`.
    #[must_use]
    pub fn with_log_out(self, log_out: Arc<dyn Departure>) -> ClientBuilder {
        ClientBuilder(Client { log_out, ..self.0 })
    }

    /// With the Wi-Fi pane following `wifi`.
    #[must_use]
    pub fn with_wifi(self, wifi: Arc<dyn WifiLink>) -> ClientBuilder {
        ClientBuilder(Client { wifi, ..self.0 })
    }

    /// With the Bluetooth pane following `bluetooth`.
    #[must_use]
    pub fn with_bluetooth(self, bluetooth: Arc<dyn BluetoothLink>) -> ClientBuilder {
        ClientBuilder(Client {
            bluetooth,
            ..self.0
        })
    }

    /// With the Displays pane following `displays`.
    #[must_use]
    pub fn with_displays(self, displays: Arc<dyn DisplaysLink>) -> ClientBuilder {
        ClientBuilder(Client { displays, ..self.0 })
    }

    /// With the engine sheet working through `desk`.
    #[must_use]
    pub fn with_engines(self, desk: Arc<dyn Desk>) -> ClientBuilder {
        ClientBuilder(Client { desk, ..self.0 })
    }

    /// With the llama.cpp note over `engines`.
    #[must_use]
    pub fn with_engine_note(self, engines: Arc<dyn EnginesLink>) -> ClientBuilder {
        ClientBuilder(Client { engines, ..self.0 })
    }

    /// Looking for coding agents on `programs_path` (the value of `PATH`).
    #[must_use]
    pub fn with_programs_path(self, programs_path: String) -> ClientBuilder {
        let search = AgentSearch {
            programs_path,
            ..self.0.search.clone()
        };
        ClientBuilder(Client { search, ..self.0 })
    }

    /// Downloading coding assistants with the program at `agents_program`.
    #[must_use]
    pub fn with_agents_program(self, agents_program: Option<PathBuf>) -> ClientBuilder {
        ClientBuilder(Client {
            agents_program,
            ..self.0
        })
    }

    /// With the Wallpaper page working through `pictures`.
    #[must_use]
    pub fn with_pictures(self, pictures: Arc<dyn Pictures>) -> ClientBuilder {
        ClientBuilder(Client { pictures, ..self.0 })
    }

    /// With the sidebar's card showing the person `person` names.
    #[must_use]
    pub fn with_person(self, person: Arc<dyn PersonLink>) -> ClientBuilder {
        ClientBuilder(Client { person, ..self.0 })
    }

    /// With the Hot Corners sheet working through `corners`.
    #[must_use]
    pub fn with_corners(self, corners: Arc<dyn CornerDesk>) -> ClientBuilder {
        ClientBuilder(Client { corners, ..self.0 })
    }

    /// With the panes that follow a daemon's live settings module looking for it in `modules`.
    #[must_use]
    pub fn with_modules(self, modules: Arc<dyn ModulesLink>) -> ClientBuilder {
        ClientBuilder(Client { modules, ..self.0 })
    }

    /// With the Accounts pane over `accounts`.
    #[must_use]
    pub fn with_accounts(self, accounts: Arc<dyn AccountsLink>) -> ClientBuilder {
        ClientBuilder(Client { accounts, ..self.0 })
    }

    /// With the Accounts pane asking the sync service through `sync`.
    #[must_use]
    pub fn with_sync(self, sync: Arc<dyn SyncLink>) -> ClientBuilder {
        ClientBuilder(Client { sync, ..self.0 })
    }

    /// With the model picker setting its rows through `picker`.
    #[must_use]
    pub fn with_picker(self, picker: Arc<dyn PickerLink>) -> ClientBuilder {
        ClientBuilder(Client { picker, ..self.0 })
    }

    /// With the Privacy page setting its rows through `privacy`.
    #[must_use]
    pub fn with_privacy(self, privacy: Arc<dyn PrivacyLink>) -> ClientBuilder {
        ClientBuilder(Client { privacy, ..self.0 })
    }

    /// With the Spaces page over `spaces`.
    #[must_use]
    pub fn with_spaces(self, spaces: Arc<dyn SpacesLink>) -> ClientBuilder {
        ClientBuilder(Client { spaces, ..self.0 })
    }

    /// With the Privacy page's portal list over `screen_sharing`.
    #[must_use]
    pub fn with_screen_sharing(self, screen_sharing: Arc<dyn ScreenSharingLink>) -> ClientBuilder {
        ClientBuilder(Client {
            screen_sharing,
            ..self.0
        })
    }

    /// With the memory consent over `memory`.
    #[must_use]
    pub fn with_memory(self, memory: Arc<dyn MemoryLink>) -> ClientBuilder {
        ClientBuilder(Client { memory, ..self.0 })
    }

    /// With the running-models list over `running`.
    #[must_use]
    pub fn with_running(self, running: Arc<dyn RunningLink>) -> ClientBuilder {
        ClientBuilder(Client { running, ..self.0 })
    }

    /// With Your computers over `computers`.
    #[must_use]
    pub fn with_computers(self, computers: Arc<dyn ComputersLink>) -> ClientBuilder {
        ClientBuilder(Client {
            computers,
            ..self.0
        })
    }

    /// With the picker's reason footer reading `reasons`.
    #[must_use]
    pub fn with_reasons(self, reasons: ReasonFeed) -> ClientBuilder {
        ClientBuilder(Client { reasons, ..self.0 })
    }
}
