//! What the accounts pane tells the person when something ended: a sign-in, a sign-out, the add
//! sheet, a secret typed too early.

use settings_app_model::{AccountsNotice, AddEnd, ReauthEnd, SignOutEnd};

/// What the account is called when the pane has no label for it.
const THIS_ACCOUNT: &str = "this account";

/// The line the person reads for `notice`.
pub(crate) fn account_notice(notice: &AccountsNotice) -> String {
    match notice {
        AccountsNotice::KeyFirst => "Set the sign-in key first, then its secret.".to_owned(),
        AccountsNotice::AddEnded(end) => add_end_line(end).unwrap_or_default().to_owned(),
        AccountsNotice::SignedIn { end, account } => {
            reauth_line(*end, account.as_deref().unwrap_or(THIS_ACCOUNT))
        }
        AccountsNotice::SignedOut { end, account } => {
            sign_out_line(*end, account.as_deref().unwrap_or(THIS_ACCOUNT))
        }
        AccountsNotice::Refused(why) => why.clone(),
    }
}

/// The line the person reads when the add sheet ended, if the end has one to say.
pub(crate) fn add_end_line(end: &AddEnd) -> Option<&'static str> {
    match end {
        AddEnd::Added | AddEnd::Closed => None,
        AddEnd::AlreadyThere(_) => Some("This account is already added."),
        AddEnd::Busy => Some("An Add Account window is already open."),
        AddEnd::Failed => Some("Could not add an account. Try again later."),
    }
}

/// The calm line for how a sign-in to the account called `label` ended.
fn reauth_line(end: ReauthEnd, label: &str) -> String {
    match end {
        ReauthEnd::Done => format!("Signed in to {label} again."),
        ReauthEnd::Cancelled => format!("Signing in to {label} was cancelled."),
        ReauthEnd::Refused => format!("The sign-in to {label} was refused."),
        ReauthEnd::NoLauncher => {
            format!("The app that runs {label} is not open. Open it and try again.")
        }
        ReauthEnd::Failed => format!("Could not sign in to {label}. Try again later."),
    }
}

/// The line for how an assistant's sign-out of the account called `label` ended.
fn sign_out_line(end: SignOutEnd, label: &str) -> String {
    match end {
        SignOutEnd::Asked => format!("Asking {label} to sign out\u{2026}"),
        SignOutEnd::LoginUntouched => {
            format!("{label} is still signed in on its own; open it to sign out there.")
        }
        SignOutEnd::Done => format!("Signed out of {label}."),
        SignOutEnd::Failed => format!("Couldn't sign out of {label}."),
        SignOutEnd::Cancelled => format!("Signing out of {label} was cancelled."),
        SignOutEnd::Expired => format!("{label} didn't answer in time."),
        SignOutEnd::LauncherGone => {
            format!("The app that runs {label} quit before signing out.")
        }
    }
}
