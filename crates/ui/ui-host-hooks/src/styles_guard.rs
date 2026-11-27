//! The crash guard for styles (customization design section 7.4).
//!
//! A bad stylesheet can stop a program from starting, and a program that cannot start cannot show
//! the switch that turns the style off. So before a window first loads styles the guard writes the
//! fingerprint of the three sheets to `<state>/bardo/styles-guard/<program>` marked `starting`, and
//! once the first frame has been drawn and the program has lived five seconds it marks the record
//! `good`. If the last two starts with the same sheets never got there, the next start is without
//! them: the app draws with `custom_styles: false` and shows
//! `ui_components::prelude::StylesOffNotice`, whose button calls [`StylesGuard::turn_on_again`]. A
//! change to any of the three sheets clears the guard ([`StylesGuard::changed`]), so the person can
//! fix the style and try again.
//!
//! The state directory is a parameter, never read from the environment here: an app passes
//! `ConfigRoot::state_dir(AppName::BARDO)`, a test passes a scratch directory. Writes are atomic
//! (a temp file renamed over the record) and best effort: a state directory that cannot be
//! written leaves the program without a guard, never without its window.

mod decide;
mod record;

pub use decide::{FAILED_STARTS, decide};
pub use record::{Record, parse, render};

use dioxus::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Duration;
use ui_components::style::kit::{Fingerprint, Styles};

/// How long a program must live after its first frame for the start to count as good.
pub const ALIVE_FOR: Duration = Duration::from_secs(5);

/// The guard of one program's start: where its record is, which sheets it covers, and what it
/// decided. A plain value; every change makes a new one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StylesGuard {
    state: PathBuf,
    program: String,
    hash: Fingerprint,
    load: bool,
}

impl StylesGuard {
    /// The start of `program` (its short name, which is also the record's file name) with
    /// `styles`: decide from the earlier starts, and write this one as `starting` unless the
    /// answer is to start without them. Call it before the first window loads the styles.
    #[must_use]
    pub fn begin(state_dir: &Path, program: &str, styles: &Styles) -> StylesGuard {
        let guard = StylesGuard {
            state: state_dir.to_owned(),
            program: program.to_owned(),
            hash: styles.fingerprint(),
            load: true,
        };
        let file = guard.file();
        let hash = styles.fingerprint();
        let earlier = std::fs::read_to_string(&file).map_or_default(|text| parse(&text));
        let load = decide(&earlier, hash);
        if load {
            let mut records = earlier;
            records.push(Record { hash, good: false });
            let keep = records.len().saturating_sub(FAILED_STARTS);
            write(&file, &records[keep..]);
        }
        StylesGuard { load, ..guard }
    }

    /// Where this program's record is.
    fn file(&self) -> PathBuf {
        self.state
            .join("styles-guard")
            .join(file_name(&self.program))
    }

    /// Whether to load the styles.
    #[must_use]
    pub fn load(&self) -> bool {
        self.load
    }

    /// The fingerprint of the sheets this guard covers.
    #[must_use]
    pub fn fingerprint(&self) -> Fingerprint {
        self.hash
    }

    /// Mark this start good, if it is still the last record. Called by [`use_styles_guard`].
    pub fn mark_good(&self) {
        if !self.load {
            return;
        }
        let file = self.file();
        let mut records = std::fs::read_to_string(&file).map_or_default(|text| parse(&text));
        if let Some(last) = records.last_mut().filter(|last| last.hash == self.hash) {
            last.good = true;
            write(&file, &records);
        }
    }

    /// The button **Turn On Again**: forget the failed starts and begin again with `styles`.
    #[must_use]
    pub fn turn_on_again(&self, styles: &Styles) -> StylesGuard {
        let _ = std::fs::remove_file(self.file());
        StylesGuard::begin(&self.state, &self.program, styles)
    }

    /// A sheet changed: if `styles` are not the ones this guard covers, clear it and begin again
    /// with them (a fix gets a fresh try); otherwise this guard stands.
    #[must_use]
    pub fn changed(&self, styles: &Styles) -> StylesGuard {
        if styles.fingerprint() == self.hash {
            return self.clone();
        }
        self.turn_on_again(styles)
    }
}

/// Mark `guard`'s start good once the window has drawn its first frame and the program has lived
/// [`ALIVE_FOR`].
///
/// Call it once in the root component, passing the guard it currently holds; a new guard (after
/// [`StylesGuard::changed`]) starts a new wait. The wait is a sleeping thread, so it needs no async
/// runtime.
#[expect(
    clippy::needless_pass_by_value,
    reason = "the hook watches the guard the caller holds this render, which `use_reactive` clones to compare"
)]
pub fn use_styles_guard(guard: StylesGuard) {
    use_effect(use_reactive!(|guard| {
        if guard.load() {
            std::thread::spawn(move || {
                std::thread::sleep(ALIVE_FOR);
                guard.mark_good();
            });
        }
    }));
}

/// The record's file name for `program`: no path separators, never hidden.
fn file_name(program: &str) -> String {
    let name = program.replace(['/', '\\'], "_");
    name.trim_start_matches('.').to_owned()
}

/// Write `records` over the file, whole or not at all.
fn write(file: &Path, records: &[Record]) {
    let Some(dir) = file.parent() else {
        return;
    };
    if std::fs::create_dir_all(dir).is_ok() {
        let _ = base_fs::write_atomic(
            file,
            render(records).as_bytes(),
            base_fs::NewFileMode::Umask,
        );
    }
}
