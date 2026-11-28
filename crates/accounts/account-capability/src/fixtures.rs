//! Capabilities and claims this crate's tests build from.

use crate::offer::{Claim, Offer, Provenance, Subject};
use crate::{
    Access, Capability, Delta, HashKind, LabelModel, MailCap, MailTransport, StorageCap,
    StorageScope,
};

/// A storage capability with the given access, change feed and reach.
#[must_use]
pub fn storage(access: Access, delta: Delta, scope: StorageScope) -> Capability {
    Capability::Storage(StorageCap {
        access,
        delta,
        quota: true,
        scope,
        hashes: HashKind::Sha1,
        ranges: true,
        chunked_upload: false,
    })
}

/// A mail capability that offers `send` as given.
#[must_use]
pub fn mail(send: bool) -> Capability {
    Capability::Mail(MailCap {
        access: Access::ReadWrite,
        send,
        delta: Delta::Push,
        transport: MailTransport::Imap,
        labels: LabelModel::Folders,
    })
}

/// A claim of `capability` made by the account itself, at `provenance`.
#[must_use]
pub fn claim(capability: Capability, provenance: Provenance) -> Claim {
    Claim {
        subject: Subject::Account,
        offer: Offer::Present(capability),
        provenance,
    }
}
