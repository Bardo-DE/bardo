//! What the Software Update page says: the standing, the actions, the sizes and the ages.

use settings_app_model::{
    ByteCount, Checked, Downloaded, ReleaseNotes, UnixSeconds, UpdateAction, UpdateStanding,
    UpdatesNotice,
};

/// The sizes a download is told in, largest first (decimal, as the Mac tells them).
const UNITS: &[(u64, &str)] = &[(1_000_000_000, "GB"), (1_000_000, "MB"), (1_000, "KB")];

const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const TWO_DAYS: i64 = 2 * DAY;

/// What the daily check is, as the page tells it.
pub(crate) fn automatic() -> &'static str {
    "Bardo checks once a day and asks before it installs anything."
}

/// The version the row is about: "Bardo 0.3.1", or "Bardo" when none is known.
pub(crate) fn title(version: Option<&str>) -> String {
    version.map_or_else(|| "Bardo".to_owned(), |version| format!("Bardo {version}"))
}

/// What is true of the version, under the title.
pub(crate) fn standing(standing: &UpdateStanding) -> String {
    match standing {
        UpdateStanding::UpToDate => "Your computer is up to date".to_owned(),
        UpdateStanding::Checking => "Checking for updates\u{2026}".to_owned(),
        UpdateStanding::Available(ByteCount(0)) => "Update available".to_owned(),
        UpdateStanding::Available(size) => {
            format!("Update available \u{b7} {}", size_words(*size))
        }
        UpdateStanding::Downloading => "Downloading\u{2026}".to_owned(),
        UpdateStanding::Preparing => "Preparing to install\u{2026}".to_owned(),
        UpdateStanding::Installing => "Installing\u{2026}".to_owned(),
        UpdateStanding::Installed => "Log out to finish the update".to_owned(),
        UpdateStanding::Failed(problem) => problem
            .clone()
            .unwrap_or_else(|| "The update did not finish. Try again in a moment.".to_owned()),
    }
}

/// The line about the last check.
pub(crate) fn checked(checked: Checked) -> String {
    match checked {
        Checked::Never => "Not checked yet".to_owned(),
        Checked::Ago(seconds) => format!("Last checked {}", elapsed_words(seconds)),
    }
}

/// The word of caution under the row, when there is one.
pub(crate) fn notice(notice: UpdatesNotice) -> Option<&'static str> {
    match notice {
        UpdatesNotice::Nothing => None,
        UpdatesNotice::NewerVersion => {
            Some("A newer version arrived. Check what's new before installing.")
        }
    }
}

/// The words on an action's button.
pub(crate) fn action(action: UpdateAction) -> &'static str {
    match action {
        UpdateAction::Check { .. } => "Check for Updates",
        UpdateAction::MoreInfo => "More Info\u{2026}",
        UpdateAction::UpdateNow => "Update Now",
        UpdateAction::TryAgain => "Try Again",
        UpdateAction::LogOut => "Log Out\u{2026}",
    }
}

/// "12.3 MB of 45.6 MB".
pub(crate) fn downloaded(downloaded: Downloaded) -> String {
    format!(
        "{} of {}",
        size_words(downloaded.done),
        size_words(downloaded.total)
    )
}

/// The notes as the sheet shows them: the release's own, or a line that there are none.
pub(crate) fn notes_body(notes: &ReleaseNotes) -> String {
    notes
        .text
        .clone()
        .unwrap_or_else(|| "No release notes came with this update.".to_owned())
}

/// `size` as a person reads it: "45.6 MB", "345 MB", "812 bytes".
fn size_words(size: ByteCount) -> String {
    let bytes = size.0;
    let Some((unit, name)) = UNITS.iter().copied().find(|(unit, _)| bytes >= *unit) else {
        return counted(i64::try_from(bytes).unwrap_or(i64::MAX), "byte");
    };
    let unit = u128::from(unit);
    let tenths = (u128::from(bytes) * 10 + unit / 2) / unit;
    if tenths >= 1000 {
        format!("{} {name}", tenths / 10)
    } else {
        format!("{}.{} {name}", tenths / 10, tenths % 10)
    }
}

/// `count` of `unit`, plural as it needs to be: "1 minute", "5 minutes".
fn counted(count: i64, unit: &str) -> String {
    if count == 1 {
        format!("{count} {unit}")
    } else {
        format!("{count} {unit}s")
    }
}

/// How long before `now` the moment `then` was: "just now", "5 minutes ago", "yesterday". A
/// moment that is not yet past (the clocks differ a little) is "just now".
pub(crate) fn age_words(now: UnixSeconds, then: UnixSeconds) -> String {
    elapsed_words(now.0.saturating_sub(then.0))
}

/// `seconds` ago, in the same words as [`age_words`], for an age a service already measured.
pub(crate) fn elapsed_words(seconds: i64) -> String {
    match seconds {
        ..MINUTE => "just now".to_owned(),
        MINUTE..HOUR => format!("{} ago", counted(seconds / MINUTE, "minute")),
        HOUR..DAY => format!("{} ago", counted(seconds / HOUR, "hour")),
        DAY..TWO_DAYS => "yesterday".to_owned(),
        _ => format!("{} ago", counted(seconds / DAY, "day")),
    }
}

/// The words "Software Update".
pub(crate) const SOFTWARE_UPDATE: &str = "Software Update";

/// The words "Automatic updates".
pub(crate) const AUTOMATIC_UPDATES: &str = "Automatic updates";

/// The words "Installing".
pub(crate) const INSTALLING: &str = "Installing";
