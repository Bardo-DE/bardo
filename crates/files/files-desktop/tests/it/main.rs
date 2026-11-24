//! files-desktop's one test binary: every file below is a module of it. The bus tests run on a
//! private `dbus-daemon` with a scratch config and fail, saying how to install it, when there is none; nothing
//! touches the person's session bus or home folder.

mod disks;
mod exec;
mod file_manager;
mod instance;
mod intent_calls;
mod intents;
mod launch;
mod storage;
mod udisks;
mod uri;
mod users;
