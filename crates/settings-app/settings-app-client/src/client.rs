//! The one door the window uses. A [`Client`] holds every service the Settings app reaches (the
//! links of `settings-app-link`); each pane's handle comes from it,
//! and a handle is a live value to draw and the commands to give it. The window never touches a
//! link, a deadline or a follow loop.

use crate::accounts::{Accounts, Listing};
use crate::bluetooth::Bluetooth;
use crate::chooser::Chooser;
use crate::computers::Computers;
use crate::displays::Displays;
use crate::engine_install::EngineInstall;
use crate::engines::{EnginePrograms, Engines};
use crate::hot_corners::Corners;
use crate::lending::Lending;
use crate::memory_consent::MemoryConsent;
use crate::picker::{AgentSearch, Models, Picker, ReasonFeed, Reasons};
use crate::privacy::Privacy;
use crate::running::Running;
use crate::screen_sharing::ScreenSharing;
use crate::spaces::Spaces;
use crate::updates::Updates;
use crate::wallpaper::Wallpaper;
use crate::whoami::Whoami;
use crate::wifi::Wifi;
use settings_app_files::{ConfigHome, DataHome};
use settings_app_link::accounts::AccountsLink;
use settings_app_link::bluetooth::BluetoothLink;
use settings_app_link::chooser::PortalLink;
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
use settings_app_link::spaces::SpacesLink;
use settings_app_link::sync::SyncLink;
use settings_app_link::updates::{Departure, UpdatesLink};
use settings_app_link::wallpaper::Pictures;
use settings_app_link::wifi::WifiLink;
use settings_app_model::{Absence, AccountsPane, AddEnd, ModuleId, UnixSeconds, Work};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

/// Every service the window reaches, behind one value that is cheap to clone.
#[derive(Debug, Clone)]
pub struct Client {
    pub(crate) modules: Arc<dyn ModulesLink>,
    pub(crate) chooser: Arc<dyn PortalLink>,
    pub(crate) accounts: Arc<dyn AccountsLink>,
    pub(crate) sync: Arc<dyn SyncLink>,
    pub(crate) picker: Arc<dyn PickerLink>,
    pub(crate) privacy: Arc<dyn PrivacyLink>,
    pub(crate) screen_sharing: Arc<dyn ScreenSharingLink>,
    pub(crate) spaces: Arc<dyn SpacesLink>,
    pub(crate) memory: Arc<dyn MemoryLink>,
    pub(crate) running: Arc<dyn RunningLink>,
    pub(crate) computers: Arc<dyn ComputersLink>,
    pub(crate) engines: Arc<dyn EnginesLink>,
    pub(crate) reasons: ReasonFeed,
    pub(crate) lender: Lender,
    pub(crate) updates: Arc<dyn UpdatesLink>,
    pub(crate) log_out: Arc<dyn Departure>,
    pub(crate) wifi: Arc<dyn WifiLink>,
    pub(crate) bluetooth: Arc<dyn BluetoothLink>,
    pub(crate) displays: Arc<dyn DisplaysLink>,
    pub(crate) desk: Arc<dyn Desk>,
    pub(crate) pictures: Arc<dyn Pictures>,
    pub(crate) person: Arc<dyn PersonLink>,
    pub(crate) corners: Arc<dyn CornerDesk>,
    pub(crate) search: AgentSearch,
    pub(crate) agents_program: Option<PathBuf>,
}

impl Client {
    /// The program that lists and downloads coding assistants, where the launcher found it.
    #[must_use]
    pub fn agents_program(&self) -> Option<&Path> {
        self.agents_program.as_deref()
    }

    /// The settings home and the data home the client reads the actions service's files from.
    #[must_use]
    pub fn homes(&self) -> (&ConfigHome, &DataHome) {
        (&self.search.config_home, &self.search.data_home)
    }

    /// Asks the bus to start the services `modules` are served by, once, as a page is shown.
    /// Nothing is started for a page that was not opened.
    pub async fn start(&self, modules: &[ModuleId]) {
        self.modules.start(modules).await;
    }

    /// The accounts the modules list now, read once; never starts a daemon.
    pub fn read_accounts(&self) -> Work<'_, Result<AccountsPane, Absence>> {
        self.accounts.read()
    }

    /// Asks accountd for the add sheet and waits for it to end.
    pub async fn add_account(&self) -> AddEnd {
        self.accounts.add_account().await
    }

    /// The time "last checked" and "allowed at" count from.
    #[must_use]
    pub fn now(&self) -> UnixSeconds {
        self.updates.now()
    }

    /// The desktop's file chooser.
    #[must_use]
    pub fn chooser(&self) -> Chooser {
        Chooser::over(Arc::clone(&self.chooser))
    }

    /// The Wi-Fi pane.
    #[must_use]
    pub fn wifi(&self) -> Wifi {
        Wifi::over(Arc::clone(&self.wifi))
    }

    /// The Bluetooth pane.
    #[must_use]
    pub fn bluetooth(&self) -> Bluetooth {
        Bluetooth::over(Arc::clone(&self.bluetooth))
    }

    /// The Displays pane.
    #[must_use]
    pub fn displays(&self) -> Displays {
        Displays::over(Arc::clone(&self.displays))
    }

    /// The Software Update pane.
    #[must_use]
    pub fn updates(&self) -> Updates {
        Updates::over(Arc::clone(&self.updates), Arc::clone(&self.log_out))
    }

    /// The Accounts pane, with the held deletions and the datasets of its accounts.
    #[must_use]
    pub fn accounts(&self) -> Accounts {
        Accounts::over(
            Arc::clone(&self.modules),
            Arc::clone(&self.accounts),
            Arc::clone(&self.sync),
        )
    }

    /// The accounts the modules list, for a page outside the Accounts pane.
    #[must_use]
    pub fn listing(&self) -> Listing {
        Listing::over(Arc::clone(&self.modules))
    }

    /// The model picker.
    #[must_use]
    pub fn picker(&self) -> Picker {
        Picker::over(
            Arc::clone(&self.modules),
            Arc::clone(&self.picker),
            Arc::clone(&self.accounts),
            self.search.clone(),
        )
    }

    /// The models inferd lists, where a place offers them to choose from.
    #[must_use]
    pub fn models(&self) -> Models {
        Models::over(Arc::clone(&self.modules))
    }

    /// The reasons the picker's footer says; handed over once, to the first caller.
    #[must_use]
    pub fn reasons(&self) -> Reasons {
        self.reasons.take()
    }

    /// The engines Local can run models with, and the download of one.
    #[must_use]
    pub fn engine_install(&self) -> EngineInstall {
        EngineInstall::over(Arc::clone(&self.desk))
    }

    /// The llama.cpp note under "Models on this computer".
    #[must_use]
    pub fn engines(&self) -> Engines {
        Engines::over(Arc::clone(&self.engines))
    }

    /// The rows that name the program of an engine.
    #[must_use]
    pub fn engine_programs(&self) -> EnginePrograms {
        EnginePrograms::over(Arc::clone(&self.modules), Arc::clone(&self.picker))
    }

    /// The models inferd runs itself.
    #[must_use]
    pub fn running(&self) -> Running {
        Running::over(Arc::clone(&self.running))
    }

    /// Your computers.
    #[must_use]
    pub fn computers(&self) -> Computers {
        Computers::over(Arc::clone(&self.computers))
    }

    /// The Privacy pane.
    #[must_use]
    pub fn privacy(&self) -> Privacy {
        Privacy::over(Arc::clone(&self.modules), Arc::clone(&self.privacy))
    }

    /// The apps that may share the screen without asking again.
    #[must_use]
    pub fn screen_sharing(&self) -> ScreenSharing {
        ScreenSharing::over(Arc::clone(&self.screen_sharing))
    }

    /// The desktop-wide Spaces.
    #[must_use]
    pub fn spaces(&self) -> Spaces {
        Spaces::over(Arc::clone(&self.spaces))
    }

    /// The looser memory settings.
    #[must_use]
    pub fn memory_consent(&self) -> MemoryConsent {
        MemoryConsent::over(Arc::clone(&self.memory))
    }

    /// The switch that lends this computer's models to the person's other computers.
    #[must_use]
    pub fn lending(&self) -> Lending {
        Lending::over(self.lender.clone())
    }

    /// The pictures behind the Wallpaper page.
    #[must_use]
    pub fn wallpaper(&self) -> Wallpaper {
        Wallpaper::over(Arc::clone(&self.pictures))
    }

    /// The person at the top of the sidebar.
    #[must_use]
    pub fn whoami(&self) -> Whoami {
        Whoami::over(Arc::clone(&self.person))
    }

    /// The Hot Corners sheet's desk.
    #[must_use]
    pub fn corners(&self) -> Corners {
        Corners::over(Arc::clone(&self.corners))
    }
}
