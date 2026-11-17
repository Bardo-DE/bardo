//! The account service, as the Accounts page meets it: the list of accounts, one account's
//! services, permissions and actions, and the sign-in keys, all over its live settings modules;
//! and the sync service's held deletions. The steps are `settings-app-model`'s `accounts_step`; an
//! effect of a row goes to the module the row came from.

mod feeds;
#[cfg(test)]
mod feeds_tests;
mod holds;
mod listing;

pub use holds::{Datasets, HeldDeletions};
pub use listing::{Listed, Listing};

use crate::follow::{Followed, follow_with};
use crate::live::Live;
use crate::notice::Notice;
use crate::pane::{Machine, Pending, pane};
use feeds::Feeds;
use settings_app_link::accounts::AccountsLink;
use settings_app_link::bus::{Daemon, plain};
use settings_app_link::modules::ModulesLink;
use settings_app_link::sync::SyncLink;
use settings_app_model::{
    AccountsEffect, AccountsInput, AccountsState, AddEnd, ModuleId, ModuleRow,
    accounts_row_is_valued, accounts_step,
};
use settings_dbus::LiveError;
use std::sync::Arc;

/// The Accounts pane's handle: cheap to clone, one live value behind it.
#[derive(Debug, Clone)]
pub struct Accounts {
    modules: Arc<dyn ModulesLink>,
    link: Arc<dyn AccountsLink>,
    feeds: Feeds,
    live: Live<AccountsState>,
    holds: HeldDeletions,
    datasets: Datasets,
}

pane!(Accounts, AccountsState, follow_all);

impl Accounts {
    pub(crate) fn over(
        modules: Arc<dyn ModulesLink>,
        link: Arc<dyn AccountsLink>,
        sync: Arc<dyn SyncLink>,
    ) -> Accounts {
        Accounts {
            holds: HeldDeletions::over(Arc::clone(&sync)),
            datasets: Datasets::over(sync),
            modules,
            link,
            feeds: Feeds::default(),
            live: Live::default(),
        }
    }

    /// Gives the pane `input` now; the returned work carries out what it asks for.
    #[must_use]
    pub fn send(&self, input: AccountsInput) -> Pending {
        self.dispatch(input)
    }

    /// The sync service's held deletions.
    #[must_use]
    pub fn holds(&self) -> &HeldDeletions {
        &self.holds
    }

    /// What the sync service says of the accounts' datasets.
    #[must_use]
    pub fn datasets(&self) -> &Datasets {
        &self.datasets
    }

    /// Asks accountd for the add sheet and waits for it to end.
    pub async fn add_account(&self) -> AddEnd {
        self.link.add_account().await
    }

    /// Each module is followed on its own: one that is not there leaves only its part out.
    async fn follow_all(&self) {
        tokio::join!(
            self.follow(ModuleId::Accountd),
            self.follow(ModuleId::Inferd),
            self.follow(ModuleId::Syncd),
        );
    }

    /// Follows `module` until the bus goes away. A module that is not there is reported at once
    /// and looked for again when it appears; the others go on meanwhile.
    async fn follow(&self, module: ModuleId) {
        let feed = self.feeds.of(module);
        follow_with(
            &*self.modules,
            &Daemon::of(module),
            accounts_row_is_valued,
            async |followed| match followed {
                Followed::Opened(opened, described) => {
                    feed.module.put(Some(opened));
                    self.dispatch(AccountsInput::Found(module, Ok(described)))
                        .await;
                }
                Followed::Gone(absence) => {
                    feed.module.put(None);
                    self.dispatch(AccountsInput::Found(module, Err(absence)))
                        .await;
                }
                Followed::Changed(path, value) => {
                    self.dispatch(AccountsInput::heard(module, path, &value))
                        .await;
                }
            },
        )
        .await;
    }
}

fn answer(row: ModuleRow, result: Result<(), LiveError>) -> AccountsInput {
    match result {
        Ok(()) => AccountsInput::Done(row),
        Err(error) => AccountsInput::Failed {
            row,
            why: plain(&error),
        },
    }
}

impl Machine for Accounts {
    type Input = AccountsInput;
    type Effect = AccountsEffect;

    fn step(state: AccountsState, input: AccountsInput) -> (AccountsState, Vec<AccountsEffect>) {
        accounts_step(state, input)
    }

    async fn run(&self, effect: AccountsEffect) -> Vec<AccountsInput> {
        let opened = |module: ModuleId| self.feeds.of(module).module.get();
        match effect {
            AccountsEffect::Tell(notice) => {
                self.live.notice(Notice::Account(notice));
                Vec::new()
            }
            AccountsEffect::OpenAddSheet => {
                let end = self.link.add_account().await;
                vec![AccountsInput::AddEnded(ModuleId::Accountd, end)]
            }
            AccountsEffect::Show(key) => vec![AccountsInput::Open(key)],
            AccountsEffect::Reload(module) => match opened(module) {
                Some(client) => {
                    let found = self.feeds.of(module).reload(&client).await;
                    found
                        .map(|found| AccountsInput::Reread(module, found))
                        .into_iter()
                        .collect()
                }
                None => Vec::new(),
            },
            AccountsEffect::Retry(module) => match opened(module) {
                Some(client) => {
                    let found = self.feeds.of(module).reload_later(&client).await;
                    found
                        .map(|found| AccountsInput::Reread(module, found))
                        .into_iter()
                        .collect()
                }
                None => Vec::new(),
            },
            AccountsEffect::Set { row, value } => match opened(row.module) {
                Some(client) => {
                    let result = client.set(&row.path, &value).await;
                    vec![answer(row, result)]
                }
                None => Vec::new(),
            },
            AccountsEffect::SetSecret { row, secret } => match opened(row.module) {
                Some(client) => {
                    let result = client
                        .set(&row.path, &toml::Value::String(secret.expose().to_owned()))
                        .await;
                    vec![answer(row, result)]
                }
                None => Vec::new(),
            },
            AccountsEffect::Invoke(row) => match opened(row.module) {
                Some(client) => {
                    let result = client.invoke(&row.path).await;
                    vec![answer(row, result)]
                }
                None => Vec::new(),
            },
        }
    }
}
