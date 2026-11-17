//! Opening and ejecting a disk.

use std::collections::HashMap;
use std::path::PathBuf;

use files_core::{EjectError, FolderPath, MountError, ProgramName, VolumeCauseError, VolumeId};

use zbus::zvariant::Value;

use super::proxies::{DriveProxy, FilesystemProxy};
use super::{BLOCK, DRIVE, Disks, FILESYSTEM, drive_of, mount_points, text_path};
use crate::error::Said;

impl Disks {
    async fn filesystem(&self, id: &VolumeId) -> zbus::Result<FilesystemProxy<'static>> {
        FilesystemProxy::builder(&self.connection)
            .path(id.as_str().to_owned())?
            .build()
            .await
    }

    /// The first folder the disk is mounted at, read from the service now.
    async fn mounted_at(&self, id: &VolumeId) -> Option<FolderPath> {
        let objects = self.objects().await.ok()?;
        let (_, interfaces) = objects
            .iter()
            .find(|(path, _)| path.as_str() == id.as_str())?;
        let (_, props) = interfaces
            .iter()
            .find(|(interface, _)| interface.as_str() == FILESYSTEM)?;
        mount_points(props)
            .into_iter()
            .find_map(|mount| FolderPath::new(mount).ok())
    }

    /// Opens the disk, giving the folder it is at. The service asks the person to authorize when
    /// the system's rules say so.
    ///
    /// # Errors
    ///
    /// `MountError::Gone` when the disk is no longer there, and `Other` when the service refuses
    /// or gives no usable folder.
    pub async fn mount(&self, id: &VolumeId) -> Result<FolderPath, MountError> {
        let filesystem = self
            .filesystem(id)
            .await
            .map_err(|error| MountError::Gone(Some(VolumeCauseError::new(error))))?;
        match filesystem.mount(HashMap::new()).await {
            Ok(path) => FolderPath::new(path)
                .map_err(|error| MountError::Other(Some(VolumeCauseError::new(error)))),
            Err(error) => match Said::from_error(&error) {
                Said::AlreadyMounted => self.mounted_at(id).await.ok_or(MountError::Other(None)),
                said @ (Said::Busy | Said::NotAllowed | Said::NotMounted | Said::Other) => {
                    Err(said.mount_failure(error))
                }
            },
        }
    }

    /// Ejects the disk: unmounts it, then switches its drive off or ejects its media. A disk
    /// something has open is left alone, and the programs that were found come with the failure.
    ///
    /// # Errors
    ///
    /// `EjectError` when the disk cannot be unmounted or its drive cannot be switched off,
    /// including when programs have it open.
    pub async fn eject(&self, id: &VolumeId) -> Result<(), EjectError> {
        self.eject_with(id, false).await
    }

    /// Ejects the disk although programs may be using it: the service unmounts it with its
    /// `force` option (a lazy unmount of what is still open), then the drive is taken away as
    /// [`eject`](Disks::eject) does. Ask the person first.
    ///
    /// # Errors
    ///
    /// `EjectError` when the disk cannot be unmounted even by force, or its drive cannot be
    /// switched off.
    pub async fn force_eject(&self, id: &VolumeId) -> Result<(), EjectError> {
        self.eject_with(id, true).await
    }

    /// `force`: unmount even though something has the disk open.
    async fn eject_with(&self, id: &VolumeId, force: bool) -> Result<(), EjectError> {
        let filesystem = self
            .filesystem(id)
            .await
            .map_err(|error| EjectError::Gone(Some(VolumeCauseError::new(error))))?;
        let mounts = self.mounts_of(id).await;
        match filesystem.unmount(unmount_options(force)).await {
            Ok(()) => {}
            Err(error) => match Said::from_error(&error) {
                Said::NotMounted => {}
                said @ (Said::Busy | Said::NotAllowed | Said::AlreadyMounted | Said::Other) => {
                    return Err(said.eject_failure(error, self.programs_using(mounts).await));
                }
            },
        }
        self.take_away(id).await
    }

    async fn mounts_of(&self, id: &VolumeId) -> Vec<PathBuf> {
        let Ok(objects) = self.objects().await else {
            return Vec::new();
        };
        objects
            .iter()
            .find(|(path, _)| path.as_str() == id.as_str())
            .and_then(|(_, interfaces)| {
                interfaces
                    .iter()
                    .find(|(interface, _)| interface.as_str() == FILESYSTEM)
            })
            .map(|(_, props)| mount_points(props))
            .unwrap_or_default()
    }

    /// The programs using any of `mounts`, searched off the bus's thread.
    async fn programs_using(&self, mounts: Vec<PathBuf>) -> Vec<ProgramName> {
        let users = std::sync::Arc::clone(&self.users);
        let found = tokio::task::spawn_blocking(move || {
            let mut all: Vec<ProgramName> =
                mounts.iter().flat_map(|mount| users.using(mount)).collect();
            all.sort();
            all.dedup();
            all
        })
        .await;
        found.unwrap_or_default()
    }

    /// After the unmount: power the drive off when it can be, else eject its media when it can be.
    async fn take_away(&self, id: &VolumeId) -> Result<(), EjectError> {
        let Some(drive) = self.drive_of(id).await else {
            return Ok(());
        };
        let proxy = DriveProxy::builder(&self.connection)
            .path(drive.path)
            .map_err(|error| EjectError::Other(Some(VolumeCauseError::new(error))))?
            .build()
            .await
            .map_err(|error| EjectError::Other(Some(VolumeCauseError::new(error))))?;
        let done = match (drive.can_power_off, drive.ejectable) {
            (true, _) => proxy.power_off(HashMap::new()).await,
            (false, true) => proxy.eject(HashMap::new()).await,
            (false, false) => return Ok(()),
        };
        done.map_err(|error| Said::from_error(&error).eject_failure(error, Vec::new()))
    }

    /// The drive behind the block device, with what it can do.
    async fn drive_of(&self, id: &VolumeId) -> Option<DriveAt> {
        let objects = self.objects().await.ok()?;
        let (_, interfaces) = objects
            .iter()
            .find(|(path, _)| path.as_str() == id.as_str())?;
        let (_, block) = interfaces
            .iter()
            .find(|(interface, _)| interface.as_str() == BLOCK)?;
        let path = text_path(block, "Drive")?;
        let (_, drive_interfaces) = objects.iter().find(|(at, _)| at.as_str() == path)?;
        let (_, props) = drive_interfaces
            .iter()
            .find(|(interface, _)| interface.as_str() == DRIVE)?;
        let facts = drive_of(props);
        Some(DriveAt {
            path,
            can_power_off: facts.can_power_off,
            ejectable: facts.ejectable,
        })
    }
}

/// A drive and what it can do after its disks are unmounted.
struct DriveAt {
    path: String,
    can_power_off: bool,
    ejectable: bool,
}

/// The options of `Filesystem.Unmount`: with `force`, the service unmounts even so.
fn unmount_options(force: bool) -> HashMap<&'static str, Value<'static>> {
    if force {
        HashMap::from([("force", Value::from(true))])
    } else {
        HashMap::new()
    }
}
