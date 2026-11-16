//! The answers as Settings lists them: the person's, and the administrator's.

use super::Privilege;
use super::book::{UnixSeconds, Verdict};
use super::subject::Subject;
use desktop_protocols::vocab::Power;

/// One program's answer for one power.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyRow {
    /// The power.
    pub power: Power,
    /// The program or app.
    pub subject: Subject,
    /// The answer.
    pub verdict: Verdict,
    /// Who set it.
    pub administrator: bool,
    /// When the person answered; none for the administrator's rows.
    pub at: Option<UnixSeconds>,
}

impl Privilege {
    /// Every row Settings lists, ordered by power, then program, then who set it.
    #[must_use]
    pub fn rows(&self) -> Vec<PrivacyRow> {
        let administrator = self.floor.rules().iter().map(|rule| PrivacyRow {
            power: rule.power,
            subject: rule.subject.clone(),
            verdict: rule.verdict,
            administrator: true,
            at: None,
        });
        let person = self.book.grants().into_iter().map(|grant| PrivacyRow {
            power: grant.power,
            subject: grant.subject,
            verdict: grant.verdict,
            administrator: false,
            at: Some(grant.at),
        });
        let mut rows: Vec<PrivacyRow> = person.chain(administrator).collect();
        rows.sort_by(|a, b| {
            (a.power, &a.subject, a.administrator).cmp(&(b.power, &b.subject, b.administrator))
        });
        rows
    }
}
