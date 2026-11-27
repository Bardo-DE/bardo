//! `SheetView` to the UI kit's `ui-shell::accounts` props: the one place the accounts service's sheet values become
//! what the drawn parts take (PLAN §2.4, design/31 §5.1). It stands in `shell-overlays`, not
//! `shell-model`: the props are the UI kit's (they name `MarkProvider` and the `Word` enums of
//! `ui-shell`), and `shell-model` may reach no design system (`cargo xtask check layers`).
//! Plain values in, plain values out; the draft supplies what the person has typed.
//!
//! | `SheetView`          | drawn as                                             |
//! | -------------------- | ---------------------------------------------------- |
//! | `Consent`            | `ConsentAlert`                                       |
//! | `Providers`          | `ProviderList`                                       |
//! | `SignIn`             | `SignInForm`                                         |
//! | `BrowserWait`        | `BrowserWait`                                        |
//! | `ShowCode`           | `ShowCode`                                           |
//! | `Review`             | `ReviewServices`                                     |
//! | `Working`            | `SignInWorking`                                      |
//! | `Failed`             | `SignInFailed`                                       |
//! | `Done`               | nothing (accountd closes the sheet)                  |

use account_core::consent::{AskPurpose, ConsentAsk};
use account_core::{AccountId, ProviderId, SessionToken};
use account_sheet::{
    Entry, FieldKind, FieldValue, ProblemKind as AccountProblem, ProviderRow, ReviewView,
    ServiceState, SheetView, SignInView,
};
use shell_model::accounts::sheet::{CopyMark, Draft, ListKey, OpenSheet};
use ui_components::components::content::provider_mark::MarkProvider;
use ui_components::prelude::Check;
use ui_shell::accounts::model::{
    AccountChoice, Attempt, Choice, ChoiceKey, FieldProblem, FieldRole, FieldText, FormField,
    FormPart, ProblemKind, ProviderEntry, ProviderKey, ProviderPick, ServiceKey, ServiceLine,
    ServiceOffer, SignInFault,
};
use ui_shell::prelude::{Recovery, WorkingFor};

use super::ranking::ranked;
use super::words::{
    ACCOUNT_LABEL, APP_NAME, absence_of, app_name, app_password_hint, choice_label, clipped,
    face_of, fault_of, limit_of, mark_of, provider_name, reached_by_other, recovery_of,
    request_phrase, service_key, short_link, usual_port, waiting_on,
};

/// What the consent alert is given.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ConsentProps {
    pub app: String,
    pub request: String,
    pub choices: Vec<AccountChoice>,
    pub chosen: Option<ChoiceKey>,
    /// The launcher session the ask was made inside: when present, "This Session Only" is offered.
    pub session: Option<SessionToken>,
    /// What the answer is for: a use of the account, or the person picking who to continue as.
    pub purpose: Purpose,
}

/// What a consent step asks, as the sheet draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Purpose {
    /// An app wants to use an account: Allow Once, Always Allow, Don't Allow.
    Use,
    /// An app offers "Continue as": pick the identity, then Continue. Nothing is granted.
    ContinueAs,
}

impl Purpose {
    /// The purpose of `ask`; a step with no account to pick from is the ordinary question, which
    /// offers "Add Account\u{2026}".
    fn of(ask: &ConsentAsk) -> Self {
        match ask.purpose {
            AskPurpose::ContinueAs if !ask.accounts.is_empty() => Self::ContinueAs,
            _ => Self::Use,
        }
    }
}

/// What the provider list is given.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ProvidersProps {
    pub providers: Vec<ProviderEntry>,
    pub query: String,
    pub cursor: Option<ProviderPick>,
}

/// What the sign-in form is given.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SignInProps {
    pub provider: String,
    pub mark: MarkProvider,
    pub fields: Vec<FormField>,
    pub problem: Option<FieldProblem>,
}

/// What the review is given.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ReviewProps {
    pub account: String,
    pub services: Vec<ServiceLine>,
    pub allow: Option<String>,
}

/// The step on screen, as props.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Step {
    Consent(ConsentProps),
    Providers(ProvidersProps),
    SignIn(SignInProps),
    Browser {
        provider: String,
        /// The address as it is drawn: the host, short. The whole one is `url`.
        shown: String,
        /// The whole address, for "Copy Link".
        url: String,
        copied: bool,
    },
    Code {
        provider: String,
        code: String,
        url: String,
        copied: bool,
    },
    Review(ReviewProps),
    Working {
        provider: String,
        waiting_for: WorkingFor,
    },
    Failed {
        provider: String,
        why: SignInFault,
        recovery: Option<Recovery>,
        mark: Option<MarkProvider>,
    },
}

impl Step {
    /// What the window's title area says: the step's own title, as the step words it.
    pub(super) fn title(&self) -> String {
        match self {
            // The alert's own headline says what the app wants; the window is named for the kind
            // of question.
            Step::Consent(props) => match props.purpose {
                Purpose::Use => "Account Access".to_owned(),
                Purpose::ContinueAs => "Continue as".to_owned(),
            },
            Step::Providers(_) => "Add Account".to_owned(),
            Step::SignIn(props) => format!("Sign in to {}", props.provider),
            Step::Browser { .. } => "Continue in your browser".to_owned(),
            Step::Code { .. } => "Enter this code".to_owned(),
            Step::Review(_) => "Choose what to use".to_owned(),
            Step::Working { provider, .. } => format!("Signing in to {provider}"),
            Step::Failed { provider, .. } => format!("Could not add {provider}"),
        }
    }
}

/// The props of `sheet`'s view, or `None` when there is nothing to draw (`Done`).
pub(super) fn step_of(sheet: &OpenSheet) -> Option<Step> {
    let draft = &sheet.draft;
    Some(match &sheet.view {
        SheetView::Consent(ask) => Step::Consent(consent_props(ask, draft)),
        SheetView::Providers(rows) => Step::Providers(providers_props(rows, draft)),
        SheetView::SignIn(form) => Step::SignIn(sign_in_props(form, draft)),
        SheetView::BrowserWait { provider, row, url } => Step::Browser {
            provider: provider_name(provider, row.as_ref()),
            shown: short_link(url),
            url: url.as_str().to_owned(),
            copied: copy_state(draft),
        },
        SheetView::ShowCode {
            provider,
            row,
            user_code,
            url,
        } => Step::Code {
            provider: provider_name(provider, row.as_ref()),
            code: user_code.0.clone(),
            url: url.as_str().to_owned(),
            copied: copy_state(draft),
        },
        SheetView::Review(review) => Step::Review(review_props(review, draft)),
        SheetView::Working { provider, row } => Step::Working {
            provider: provider_name(provider, row.as_ref()),
            waiting_for: waiting_on(row.as_ref()),
        },
        SheetView::Failed {
            provider,
            row,
            fault,
        } => Step::Failed {
            provider: provider_name(provider, row.as_ref()),
            why: fault_of(*fault),
            recovery: recovery_of(*fault),
            mark: Some(mark_of(provider.as_str())),
        },
        SheetView::Done => return None,
    })
}

/// The consent step's props.
fn consent_props(ask: &ConsentAsk, draft: &Draft) -> ConsentProps {
    ConsentProps {
        app: clipped(
            &app_name(
                &ask.app,
                ask.app_label.as_ref().map(|label| label.0.as_str()),
            ),
            APP_NAME,
        ),
        request: request_phrase(ask.kind, ask.class, ask.usage),
        choices: ask
            .accounts
            .iter()
            .map(|choice| AccountChoice {
                key: ChoiceKey(choice.account.as_str().to_owned()),
                label: clipped(&choice.label.0, ACCOUNT_LABEL),
                provider: mark_of(choice.provider.as_str()),
            })
            .collect(),
        chosen: draft
            .chosen
            .as_ref()
            .map(|account| ChoiceKey(account.as_str().to_owned())),
        session: ask.session.clone(),
        purpose: Purpose::of(ask),
    }
}

/// The provider list step's props.
fn providers_props(rows: &[ProviderRow], draft: &Draft) -> ProvidersProps {
    ProvidersProps {
        // The generic mail rows are ProviderList's own "Other…" (and the server form's
        // protocol pop-up), so they are not listed twice; the other generic rows are.
        providers: ranked(
            rows.iter()
                .filter(|row| !reached_by_other(row))
                .map(|row| {
                    let entry = ProviderEntry::new(
                        ProviderKey(row.id.as_str().to_owned()),
                        row.label.clone(),
                        mark_of(&row.mark),
                    );
                    match face_of(row) {
                        Some(face) => entry.faced(face),
                        None => entry,
                    }
                })
                .collect(),
            &draft.query,
        ),
        query: draft.query.clone(),
        cursor: draft.cursor.as_ref().map(|key| match key {
            ListKey::Provider(id) => ProviderPick::Provider(ProviderKey(id.as_str().to_owned())),
            ListKey::Other => ProviderPick::Other,
        }),
    }
}

/// The sign-in form step's props.
fn sign_in_props(form: &SignInView, draft: &Draft) -> SignInProps {
    SignInProps {
        provider: provider_name(&form.provider, form.row.as_ref()),
        mark: mark_of(form.provider.as_str()),
        fields: form_fields(draft, &form.provider),
        // A refusal from accountd first; else what cannot be right in what is typed (a
        // port that is no number), marked on its field as it is typed. An empty required
        // field is not marked: Continue says it.
        problem: form
            .problem
            .map(|problem| (problem.field, problem.problem))
            .or_else(|| {
                draft
                    .problem(&form.fields)
                    .filter(|problem| problem.problem == AccountProblem::Invalid)
                    .map(|problem| (problem.field, problem.problem))
            })
            .map(|(field, problem)| FieldProblem {
                role: role_of(field),
                kind: match problem {
                    AccountProblem::Missing => ProblemKind::Missing,
                    AccountProblem::Refused => ProblemKind::Refused,
                    AccountProblem::Invalid => ProblemKind::Invalid,
                },
                attempt: Attempt(draft.submits.0),
            }),
    }
}

/// The review step's props.
fn review_props(review: &ReviewView, draft: &Draft) -> ReviewProps {
    ReviewProps {
        account: clipped(&review.review.label.0, ACCOUNT_LABEL),
        services: review
            .review
            .services
            .iter()
            .map(|row| ServiceLine {
                key: ServiceKey(service_key(row.kind)),
                name: row.kind.name().to_owned(),
                offer: match row.state {
                    ServiceState::Offered(shipped) => {
                        ServiceOffer::Offered(switch(draft.toggle_of(row.kind).unwrap_or(shipped)))
                    }
                    ServiceState::Absent(reason) => ServiceOffer::Absent(absence_of(reason)),
                },
                limit: row.limit.map(limit_of),
            })
            .collect(),
        allow: review.allow.as_ref().map(|app| {
            let resolved = review.allow_label.as_ref().map(|label| label.0.as_str());
            clipped(&app_name(app, resolved), APP_NAME)
        }),
    }
}

/// The form's fields as drawn: what each holds, its choices, its hint and its group.
fn form_fields(draft: &Draft, provider: &ProviderId) -> Vec<FormField> {
    let held = |kind| {
        draft
            .fields
            .iter()
            .find(|field| field.kind == kind)
            .and_then(|field| match &field.value {
                FieldValue::Plain(text) => Some(text.as_str()),
                FieldValue::Secret(_) => None,
            })
    };
    let protocol = held(FieldKind::Protocol).unwrap_or("imap");
    // Only the hand-typed server form is grouped; the others stay one plain list.
    let manual = draft.fields.iter().any(|f| f.kind == FieldKind::Protocol);
    let security = |kind| held(kind).unwrap_or("tls");
    draft
        .fields
        .iter()
        .map(|field| {
            let base = FormField::new(
                role_of(field.kind),
                field.required,
                match (&field.value, field.entry) {
                    (FieldValue::Secret(text), _) => FieldText::Secret(text.clone()),
                    (FieldValue::Plain(text), Entry::Plain | Entry::Secret) => {
                        FieldText::Plain(text.clone())
                    }
                },
            );
            let base = match field.kind.choices() {
                [] => base,
                slugs => base.choosing(
                    slugs
                        .iter()
                        .map(|slug| Choice::new(*slug, choice_label(slug)))
                        .collect(),
                ),
            };
            let base = match field.kind {
                FieldKind::Port => base
                    .hinted(usual_port(protocol, false, security(FieldKind::Security)).to_string()),
                FieldKind::OutgoingPort => base.hinted(
                    usual_port(protocol, true, security(FieldKind::OutgoingSecurity)).to_string(),
                ),
                FieldKind::AppPassword => match app_password_hint(provider) {
                    Some(hint) => base.hinted(hint),
                    None => base,
                },
                _ => base,
            };
            match part_of(field.kind).filter(|_| manual) {
                Some(part) => base.in_part(part),
                None => base,
            }
        })
        .collect()
}

/// The group a field of the hand-typed server form sits in; the other forms have none.
fn part_of(kind: FieldKind) -> Option<FormPart> {
    match kind {
        FieldKind::Protocol
        | FieldKind::Server
        | FieldKind::Port
        | FieldKind::Security
        | FieldKind::SessionUrl => Some(FormPart::Incoming),
        FieldKind::OutgoingServer | FieldKind::OutgoingPort | FieldKind::OutgoingSecurity => {
            Some(FormPart::Outgoing)
        }
        FieldKind::Username | FieldKind::Password | FieldKind::AppPassword | FieldKind::Token => {
            Some(FormPart::SignIn)
        }
        // A server address (the accounts service's ServerUrl) and any kind the accounts service adds later: no group.
        _ => None,
    }
}

fn switch(toggle: bool) -> Check {
    if toggle { Check::On } else { Check::Off }
}

fn copy_state(draft: &Draft) -> bool {
    draft.copied == CopyMark::Copied
}

/// The role an accounts service field kind plays in the form.
pub(super) fn role_of(kind: FieldKind) -> FieldRole {
    match kind {
        FieldKind::Address => FieldRole::Address,
        FieldKind::Username => FieldRole::Username,
        FieldKind::Password => FieldRole::Password,
        FieldKind::AppPassword => FieldRole::AppPassword,
        FieldKind::ApiKey => FieldRole::ApiKey,
        FieldKind::Token => FieldRole::Token,
        FieldKind::Protocol => FieldRole::Protocol,
        FieldKind::Port => FieldRole::Port,
        FieldKind::Security => FieldRole::Security,
        FieldKind::OutgoingServer => FieldRole::OutgoingServer,
        FieldKind::OutgoingPort => FieldRole::OutgoingPort,
        FieldKind::OutgoingSecurity => FieldRole::OutgoingSecurity,
        FieldKind::SessionUrl => FieldRole::SessionUrl,
        FieldKind::ServerUrl => FieldRole::ServerUrl,
        // A name has no role of its own in the UI kit's form yet, so it is the plain server field.
        FieldKind::Server | FieldKind::Name => FieldRole::Server,
    }
}

/// The field kind a form role stands for (the way back).
pub(super) fn kind_of(role: FieldRole) -> FieldKind {
    match role {
        FieldRole::Address => FieldKind::Address,
        FieldRole::Server => FieldKind::Server,
        FieldRole::Username => FieldKind::Username,
        FieldRole::Password => FieldKind::Password,
        FieldRole::AppPassword => FieldKind::AppPassword,
        FieldRole::ApiKey => FieldKind::ApiKey,
        FieldRole::Token => FieldKind::Token,
        FieldRole::Protocol => FieldKind::Protocol,
        FieldRole::Port => FieldKind::Port,
        FieldRole::Security => FieldKind::Security,
        FieldRole::OutgoingServer => FieldKind::OutgoingServer,
        FieldRole::OutgoingPort => FieldKind::OutgoingPort,
        FieldRole::OutgoingSecurity => FieldKind::OutgoingSecurity,
        FieldRole::SessionUrl => FieldKind::SessionUrl,
        FieldRole::ServerUrl => FieldKind::ServerUrl,
    }
}

/// The account a consent key names, when it is one.
pub(super) fn account_of(key: &ChoiceKey) -> Option<AccountId> {
    AccountId::parse(&key.0).ok()
}

/// The provider a list key names, when it is one.
pub(super) fn provider_of(key: &ProviderKey) -> Option<ProviderId> {
    ProviderId::parse(&key.0).ok()
}
