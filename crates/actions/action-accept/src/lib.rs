//! Test only: the agent tier's first end-to-end acceptance. The real `actiond`, `companiond`,
//! `readerd`, `memoryd` and `inferd` run as processes on a private bus (inferd playing a cassette),
//! with a mail provider and a `Confirm1` server around them (see `dev/accept/README.md`).
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

pub mod acp;
pub mod catalog;
pub mod confirm;
pub mod drive;
pub mod grants;
pub mod keys_home;
pub mod provider;
pub mod runlink;
pub mod seams;
pub mod things;
pub mod world;
