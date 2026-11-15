//! What the AI broker (inferd) asks of the daemon that keeps the AI keys
//! (`org.quire.Accounts1.Broker`): which accounts an app holds a permit on, and the key behind a
//! permit. inferd is the one daemon that sends a key to an AI company, so the calls are its alone:
//! any other caller is refused. A key arrives on a sealed memfd, once, and is read into a
//! [`SecretText`], which shows nothing of itself.
//!
//! ```ignore
//! let accounts = bus.ai_accounts();
//! let mut changes = accounts.changes().await?;                 // subscribe first
//! for row in accounts.verdicts(&app, DataClass::Prompt, Usage::Interactive).await? {
//!     if let Verdict::Granted { grant, .. } = row.verdict {
//!         let key = accounts.key(&grant).await?;               // for one request, then dropped
//!     }
//! }
//! ```

use crate::Bus;
use crate::failure::{ai_accounts_error, link_error};
use futures_util::StreamExt;
use porter_capability::Claim;
use porter_client::{AiAccountsError, Changes, LinkError, Malformed};
use porter_core::consent::Usage;
use porter_core::{AccountId, AccountState, AiAccountRow, AppId, DataClass, GrantId};
use porter_core::{LlmVerdict, OwnServerRow, ProviderId, SecretText};
use porter_dbus::accounts::{BrokerProxy, ai_account_from_dbus};
use porter_dbus::accounts::{own_server_from_dbus, verdict_from_dbus};
use porter_ids::word::word as word_of;
use std::io::Read;
use zeroize::Zeroizing;

/// The most a key may hold, as the client reads one.
const KEY_LIMIT: u64 = 16 * 1024;

fn malformed(what: Malformed) -> AiAccountsError {
    AiAccountsError::Link(LinkError::Malformed(what))
}

impl Bus {
    async fn broker(&self) -> Result<BrokerProxy<'static>, AiAccountsError> {
        self.accounts
            .proxy(&self.connection)
            .await
            .map_err(|e| AiAccountsError::Link(link_error(&e)))
    }
}

/// A key read from the descriptor it arrived on.
fn read_key(fd: porter_dbus::zvariant::OwnedFd) -> Result<SecretText, AiAccountsError> {
    let owned: std::os::fd::OwnedFd = fd.into();
    let mut text = Zeroizing::new(String::new());
    std::fs::File::from(owned)
        .take(KEY_LIMIT + 1)
        .read_to_string(&mut text)
        .map_err(|_| malformed(Malformed::Descriptor))?;
    if u64::try_from(text.len()).map_or(true, |len| len > KEY_LIMIT) {
        return Err(malformed(Malformed::Descriptor));
    }
    Ok(SecretText::new(text.as_str()))
}

/// The AI accounts of accountd, as the AI broker asks them, over the bus.
#[derive(Debug, Clone)]
pub struct AiAccounts {
    bus: Bus,
}

impl Bus {
    /// The handle on ai accounts.
    pub fn ai_accounts(&self) -> AiAccounts {
        AiAccounts { bus: self.clone() }
    }
}

impl AiAccounts {
    /// What the permits say for `app` on each account that serves language models, for data of
    /// `class` used as `usage` says.
    pub async fn verdicts(
        &self,
        app: &AppId,
        class: DataClass,
        usage: Usage,
    ) -> Result<Vec<LlmVerdict>, AiAccountsError> {
        let who = (
            app.name.as_str().to_owned(),
            word_of(&app.isolation).to_owned(),
        );
        self.bus
            .broker()
            .await?
            .verdicts(&who, word_of(&class), word_of(&usage))
            .await
            .map_err(|e| ai_accounts_error(&e))?
            .into_iter()
            .map(|row| verdict_from_dbus(row).map_err(|_| malformed(Malformed::Row)))
            .collect()
    }

    /// The key behind the allow permit `grant` of the language-model kind, read once; an
    /// allow-once permit is spent by a read that succeeded. Hold it for one request and drop it.
    pub async fn key(&self, grant: &GrantId) -> Result<SecretText, AiAccountsError> {
        read_key(
            self.bus
                .broker()
                .await?
                .read_key(grant.as_str())
                .await
                .map_err(|e| ai_accounts_error(&e))?,
        )
    }

    /// The AI accounts (the services, the programs on this computer and the assistants), for the
    /// rows the broker adds to them.
    pub async fn accounts(&self) -> Result<Vec<AiAccountRow>, AiAccountsError> {
        self.bus
            .broker()
            .await?
            .accounts()
            .await
            .map_err(|e| ai_accounts_error(&e))?
            .into_iter()
            .map(|row| ai_account_from_dbus(row).map_err(|_| malformed(Malformed::Row)))
            .collect()
    }

    /// The accounts that are servers of the person's own.
    pub async fn servers(&self) -> Result<Vec<OwnServerRow>, AiAccountsError> {
        self.bus
            .broker()
            .await?
            .servers()
            .await
            .map_err(|e| ai_accounts_error(&e))?
            .into_iter()
            .map(|row| own_server_from_dbus(row).map_err(|_| malformed(Malformed::Row)))
            .collect()
    }

    /// The key of the own server `account`, for the one request that asks it for its models. It
    /// needs no permit.
    pub async fn server_key(&self, account: &AccountId) -> Result<SecretText, AiAccountsError> {
        read_key(
            self.bus
                .broker()
                .await?
                .read_server_key(account.as_str())
                .await
                .map_err(|e| ai_accounts_error(&e))?,
        )
    }

    /// Reports a probed local runtime as the account of `provider`, its models as `claims`, in
    /// `state` (`Ok` or `Offline`); the account's id.
    pub async fn report_local(
        &self,
        provider: &ProviderId,
        claims: &[Claim],
        state: AccountState,
    ) -> Result<AccountId, AiAccountsError> {
        let claims =
            serde_json::to_string(claims).map_err(|_| AiAccountsError::Link(LinkError::Invalid))?;
        let id = self
            .broker()
            .await?
            .report_local(provider.as_str(), &claims, word_of(&state))
            .await
            .map_err(|e| ai_accounts_error(&e))?;
        AccountId::parse(&id).map_err(|_| malformed(Malformed::Id))
    }

    /// "An AI account, a permit or a switch changed", from now on; each item says only that
    /// something moved, and the listener asks again. Subscribe before the first look, so a change
    /// between the look and the subscription is not missed.
    pub async fn changes(&self) -> Result<Changes<()>, AiAccountsError> {
        let stream = self
            .broker()
            .await?
            .receive_changed()
            .await
            .map_err(|e| ai_accounts_error(&e))?;
        Ok(Changes::of(stream.map(|_| ())))
    }
}
