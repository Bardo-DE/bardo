//! Every service the Settings window reaches, as one value: what a client is built over.

use crate::accounts::AccountsLink;
use crate::bluetooth::BluetoothLink;
use crate::chooser::PortalLink;
use crate::computers::ComputersLink;
use crate::displays::DisplaysLink;
use crate::engine_install::Desk;
use crate::engines::EnginesLink;
use crate::hot_corners::CornerDesk;
use crate::lending::Lender;
use crate::memory_consent::MemoryLink;
use crate::modules::ModulesLink;
use crate::person::PersonLink;
use crate::picker::PickerLink;
use crate::privacy::PrivacyLink;
use crate::running::RunningLink;
use crate::screen_sharing::ScreenSharingLink;
use crate::spaces::SpacesLink;
use crate::sync::SyncLink;
use crate::updates::{Departure, UpdatesLink};
use crate::wallpaper::Pictures;
use crate::wifi::WifiLink;
use std::sync::Arc;

/// The services the window reaches, each behind its link; cheap to clone.
#[derive(Debug, Clone)]
pub struct Services {
    /// The desktop's file chooser.
    pub chooser: Arc<dyn PortalLink>,
    /// The live settings modules of the daemons.
    pub modules: Arc<dyn ModulesLink>,
    /// The account service.
    pub accounts: Arc<dyn AccountsLink>,
    /// The sync service.
    pub sync: Arc<dyn SyncLink>,
    /// The model picker's rows.
    pub picker: Arc<dyn PickerLink>,
    /// The Privacy page's rows.
    pub privacy: Arc<dyn PrivacyLink>,
    /// The screen-sharing portal.
    pub screen_sharing: Arc<dyn ScreenSharingLink>,
    /// The desktop-wide Spaces.
    pub spaces: Arc<dyn SpacesLink>,
    /// Memory's consent.
    pub memory: Arc<dyn MemoryLink>,
    /// The models inferd runs itself.
    pub running: Arc<dyn RunningLink>,
    /// Your computers.
    pub computers: Arc<dyn ComputersLink>,
    /// The llama.cpp note.
    pub engines: Arc<dyn EnginesLink>,
    /// The lending switch.
    pub lender: Lender,
    /// The update service.
    pub updates: Arc<dyn UpdatesLink>,
    /// Logging out.
    pub log_out: Arc<dyn Departure>,
    /// The network service behind Wi-Fi.
    pub wifi: Arc<dyn WifiLink>,
    /// BlueZ.
    pub bluetooth: Arc<dyn BluetoothLink>,
    /// The compositor's displays.
    pub displays: Arc<dyn DisplaysLink>,
    /// The engine sheet.
    pub desk: Arc<dyn Desk>,
    /// The Wallpaper page's pictures.
    pub pictures: Arc<dyn Pictures>,
    /// Who is using this computer.
    pub person: Arc<dyn PersonLink>,
    /// The Hot Corners sheet.
    pub corners: Arc<dyn CornerDesk>,
}
