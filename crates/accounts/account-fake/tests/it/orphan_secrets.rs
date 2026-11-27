//! The startup sweep of orphan secrets, over the in-memory store only.

use account_core::{AccountId, Credential, SecretKey, SecretPurpose, SecretText};
use account_fake::{RecordingAudit, mail_account};
use account_secrets::{MemorySecrets, Secrets};
use account_service_accounts::sweep_orphan_secrets;
use account_service_core::audit::AuditEvent;
use account_service_core::registry::Registry;
use base_time::FixedClock;
use base_time::UnixSeconds;

fn key(id: &AccountId) -> SecretKey {
    SecretKey {
        account: id.clone(),
        purpose: SecretPurpose::Password,
    }
}

async fn file(secrets: &MemorySecrets, id: &AccountId) {
    let value = Credential::Password(SecretText::new("pw".to_owned()));
    secrets.put(&key(id), &value).await.expect("put");
}

fn registry_of(ids: &[&AccountId]) -> Registry {
    let mut registry = Registry::default();
    for id in ids {
        let mut account = mail_account();
        account.id = (*id).clone();
        registry.accounts.push(account);
    }
    registry
}

#[tokio::test]
async fn only_secrets_the_registry_does_not_name_are_swept_and_each_is_audited() {
    let (kept, orphan) = (
        AccountId::parse("kept").expect("id"),
        AccountId::parse("orphan").expect("id"),
    );
    let secrets = MemorySecrets::default();
    file(&secrets, &kept).await;
    file(&secrets, &orphan).await;
    let audit = RecordingAudit::default();
    let clock = FixedClock::new(UnixSeconds(7));
    let swept = sweep_orphan_secrets(&secrets, &registry_of(&[&kept]), &audit, &clock).await;
    assert_eq!(swept, vec![orphan.clone()]);
    secrets.get(&key(&kept)).await.unwrap();
    secrets.get(&key(&orphan)).await.unwrap_err();
    let entries = audit.entries();
    let [entry] = entries.as_slice() else {
        panic!("one line: {entries:?}")
    };
    assert_eq!(entry.event, AuditEvent::SecretsSwept);
    assert_eq!(entry.account.as_ref(), Some(&orphan));
    assert_eq!(entry.at, UnixSeconds(7));
}

#[tokio::test]
async fn an_empty_registry_sweeps_nothing() {
    let id = AccountId::parse("lost").expect("id");
    let secrets = MemorySecrets::default();
    file(&secrets, &id).await;
    let audit = RecordingAudit::default();
    let clock = FixedClock::new(UnixSeconds(7));
    let swept = sweep_orphan_secrets(&secrets, &Registry::default(), &audit, &clock).await;
    assert!(swept.is_empty());
    secrets.get(&key(&id)).await.unwrap();
    assert!(audit.entries().is_empty());
}
