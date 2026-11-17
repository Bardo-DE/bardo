//! What every flow starts from: a client builder over no bus, with its files in a folder nothing
//! reads or writes.

use settings_app_client::{ClientBuilder, ClientHomes};
use settings_app_files::{CacheHome, ConfigHome, DataHome};
use settings_app_link::bus::BusSource;
use settings_app_system::services;
use std::path::PathBuf;

/// A builder whose every service is the real one until a test replaces it; the bus is absent, so
/// nothing it holds is ever reached.
pub fn builder() -> ClientBuilder {
    let nowhere = PathBuf::from("/nonexistent/detent-client-tests");
    let data = DataHome(nowhere.join("data"));
    let real = services::over(
        &BusSource::Absent,
        &data,
        &CacheHome(nowhere.join("cache")),
        Vec::new(),
    );
    ClientBuilder::over(
        real,
        ClientHomes {
            config: ConfigHome(nowhere.join("config")),
            data,
        },
    )
}
