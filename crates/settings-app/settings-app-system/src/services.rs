//! Every service over the real bus: what a client is built over when nothing is replaced.

use crate::accounts::OnBus as AccountsOnBus;
use crate::bluetooth::OnBluetooth;
use crate::chooser::OnPortal;
use crate::computers::OnBus as ComputersOnBus;
use crate::displays::OnCompositor;
use crate::engine_install::OnEngines;
use crate::engines::OnBus as EnginesOnBus;
use crate::hot_corners::OnCorners;
use crate::lending;
use crate::memory_consent::OnBus as MemoryOnBus;
use crate::modules::OnBus as ModulesOnBus;
use crate::person::OnAccounts;
use crate::picker::OnClient as PickerOnClient;
use crate::privacy::OnClient as PrivacyOnClient;
use crate::running::OnBus as RunningOnBus;
use crate::screen_sharing::OnBus as ScreenSharingOnBus;
use crate::spaces::OnBus as SpacesOnBus;
use crate::sync::OnBus as SyncOnBus;
use crate::updates::{OnBus as UpdatesOnBus, PowerMenu};
use crate::wallpaper::{OnDisk, Thumbnails};
use crate::wifi::OnWifi;
use base_xdg::Xdg;
use settings_app_files::{CacheHome, DataHome, PictureDirs};
use settings_app_link::bus::BusSource;
use settings_app_link::person::{NoPerson, PersonLink};
use settings_app_link::services::Services;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where the person's name and picture come from when the services are looked for on `bus`: on
/// the person's own session, the system's accounts, else the passwd file and the home folder. On
/// no bus, or a test's private one, nobody is known, so nothing of this computer's own reaches a
/// window that was told to look somewhere else.
fn person_on(bus: &BusSource) -> Arc<dyn PersonLink> {
    match bus {
        BusSource::Session => Arc::new(OnAccounts::new(
            bus.clone(),
            PathBuf::from("/etc/passwd"),
            Xdg::process()
                .home()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
        )),
        BusSource::Absent | BusSource::Address(..) => Arc::new(NoPerson),
    }
}

/// The real services, looked for on `bus`: docket's agents and the added pictures under `data`,
/// the Wallpaper page's small pictures under `cache`, and the apps' actions listed from the
/// manifests in `intent_dirs`, in the order of trust.
#[must_use]
pub fn over(
    bus: &BusSource,
    data: &DataHome,
    cache: &CacheHome,
    intent_dirs: Vec<PathBuf>,
) -> Services {
    let chooser = Arc::new(OnPortal(bus.clone()));
    let desk = Arc::new(OnEngines::new(bus.clone(), Arc::clone(&chooser), &data.0));
    let pictures = Arc::new(OnDisk::new(
        Arc::clone(&chooser),
        PictureDirs::standard(data),
        Thumbnails::under(cache),
    ));
    let corners = Arc::new(OnCorners::new(Arc::clone(&chooser), intent_dirs));
    Services {
        chooser,
        modules: Arc::new(ModulesOnBus(bus.clone())),
        accounts: Arc::new(AccountsOnBus(bus.clone())),
        sync: Arc::new(SyncOnBus(bus.clone())),
        picker: Arc::new(PickerOnClient),
        privacy: Arc::new(PrivacyOnClient),
        screen_sharing: Arc::new(ScreenSharingOnBus(bus.clone())),
        spaces: Arc::new(SpacesOnBus(bus.clone())),
        memory: Arc::new(MemoryOnBus(bus.clone())),
        running: Arc::new(RunningOnBus(bus.clone())),
        computers: Arc::new(ComputersOnBus(bus.clone())),
        engines: Arc::new(EnginesOnBus(bus.clone())),
        lender: lending::from_env(),
        updates: Arc::new(UpdatesOnBus(bus.clone())),
        log_out: Arc::new(PowerMenu),
        wifi: Arc::new(OnWifi(bus.clone())),
        bluetooth: Arc::new(OnBluetooth::new(bus.clone())),
        displays: Arc::new(OnCompositor(bus.clone())),
        desk,
        pictures,
        person: person_on(bus),
        corners,
    }
}
