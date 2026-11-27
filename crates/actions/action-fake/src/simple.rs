//! The small fakes: a recording event sink and an in-memory consent store.

use action_core::AuditRecord;
use action_core::EventSink;
use action_router::GrantStore;
use permission_core::{
    ActionGrant, Limit, RuleFile, StandingGrant, StandingGrantId, held_with, held_without,
    limits_with, limits_without, with_rule, without_rule,
};
use permission_policy::Presets;
use std::sync::Mutex;

/// An event sink that keeps every record, in order.
#[derive(Debug, Default)]
pub struct RecordingSink {
    records: Mutex<Vec<AuditRecord>>,
}

impl RecordingSink {
    /// An empty sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything appended so far.
    pub fn records(&self) -> Vec<AuditRecord> {
        self.records.lock().map_or_default(|r| r.clone())
    }

    /// Forgets everything appended.
    pub fn clear(&self) {
        if let Ok(mut records) = self.records.lock() {
            records.clear();
        }
    }
}

impl EventSink for RecordingSink {
    fn append(&self, record: AuditRecord) {
        if let Ok(mut records) = self.records.lock() {
            records.push(record);
        }
    }
}

/// The text of the two rules files a store writes.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RuleTexts {
    /// The person's `never.cedar`.
    pub never: String,
    /// The assistant's `assistant.cedar`.
    pub assistant: String,
}

impl RuleTexts {
    fn file(&mut self, file: RuleFile) -> &mut String {
        match file {
            RuleFile::Never => &mut self.never,
            RuleFile::Assistant => &mut self.assistant,
        }
    }
}

/// A consent store in memory.
#[derive(Debug, Default)]
pub struct MemoryGrants {
    grants: Mutex<Vec<ActionGrant>>,
    standing: Mutex<Vec<StandingGrant>>,
    limits: Mutex<Vec<Limit>>,
    rules: Mutex<RuleTexts>,
}

impl MemoryGrants {
    /// A store with no grants.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets every grant, the standing ones too.
    pub fn clear(&self) {
        if let Ok(mut grants) = self.grants.lock() {
            grants.clear();
        }
        if let Ok(mut standing) = self.standing.lock() {
            standing.clear();
        }
        if let Ok(mut limits) = self.limits.lock() {
            limits.clear();
        }
        if let Ok(mut rules) = self.rules.lock() {
            *rules = RuleTexts::default();
        }
    }

    /// The rules files as written so far.
    pub fn rule_texts(&self) -> RuleTexts {
        self.rules.lock().map_or_default(|r| r.clone())
    }

    /// A store that starts with these grants.
    #[must_use]
    pub fn with(grants: Vec<ActionGrant>) -> Self {
        Self {
            grants: Mutex::new(grants),
            standing: Mutex::default(),
            limits: Mutex::default(),
            rules: Mutex::default(),
        }
    }
}

impl GrantStore for MemoryGrants {
    fn grants(&self) -> Vec<ActionGrant> {
        self.grants.lock().map_or_default(|g| g.clone())
    }

    fn record(&self, grant: ActionGrant) {
        if let Ok(mut grants) = self.grants.lock() {
            grants.push(grant);
        }
    }

    fn standing(&self) -> Vec<StandingGrant> {
        self.standing.lock().map_or_default(|g| g.clone())
    }

    fn add_standing(&self, grant: StandingGrant) {
        if let Ok(mut held) = self.standing.lock() {
            *held = held_with(std::mem::take(&mut held), grant);
        }
    }

    fn revoke_standing(&self, id: &StandingGrantId) -> bool {
        if let Ok(mut held) = self.standing.lock() {
            let (rest, done) = held_without(std::mem::take(&mut held), id);
            *held = rest;
            if done {
                return done;
            }
        }
        if let Ok(mut held) = self.limits.lock() {
            let (rest, done) = limits_without(std::mem::take(&mut held), id);
            *held = rest;
            if done {
                return done;
            }
        }
        let Ok(mut rules) = self.rules.lock() else {
            return false;
        };
        [RuleFile::Never, RuleFile::Assistant]
            .into_iter()
            .map(|file| {
                let (rest, done) = without_rule(&*rules.file(file), id);
                *rules.file(file) = rest;
                done
            })
            .find(|done| *done)
            .unwrap_or(false)
    }

    fn add_rule(&self, limit: &Limit) {
        let (Some(file), Ok(mut rules)) = (RuleFile::of(limit.by), self.rules.lock()) else {
            return;
        };
        let text = rules.file(file);
        *text = with_rule(&*text, limit);
    }

    fn rebuilt(&self) -> Option<Presets> {
        let rules = self.rule_texts();
        Presets::with_rules(&rules.never, &rules.assistant).ok()
    }

    fn limits(&self) -> Vec<Limit> {
        self.limits.lock().map_or_default(|l| l.clone())
    }

    fn add_limit(&self, limit: Limit) {
        if let Ok(mut held) = self.limits.lock() {
            *held = limits_with(std::mem::take(&mut held), limit);
        }
    }
}
