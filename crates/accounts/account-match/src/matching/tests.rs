use super::*;
use crate::need::AgentNeed;
use crate::need::{ComputerUseNeed, LlmNeed, MailNeed, PhotosNeed, StorageNeed};
use account_capability::{
    Access, Albums, CapabilityKind, Delta, LibraryRead, LlmFeature, PhotosCap, StorageScope,
};
use account_capability::{AgentCap, AgentProgram, AgentProtocol, EnvName};
use account_capability::{ComputerUseBatching, ComputerUseCap, ComputerUseEnv, LlmWire};
use account_capability::{HashKind, LabelModel, LlmCap, MailCap, MailTransport, StorageCap};
use base_ids::{Px, Tokens};

/// A storage capability with the given access, change feed and reach.
fn storage(access: Access, delta: Delta, scope: StorageScope) -> Capability {
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
fn mail(send: bool) -> Capability {
    Capability::Mail(MailCap {
        access: Access::ReadWrite,
        send,
        delta: Delta::Push,
        transport: MailTransport::Imap,
        labels: LabelModel::Folders,
    })
}

/// A language model with the given features and context window.
fn llm(features: &[LlmFeature], context: u32) -> Capability {
    Capability::Llm(LlmCap {
        features: features.iter().copied().collect(),
        context: Tokens(context),
        max_output: Tokens(4096),
        wire: LlmWire::ChatCompletions,
    })
}

fn agent(program: &str, base_url_env: Option<&str>, protocols: &[AgentProtocol]) -> Offer {
    present(Capability::Agent(Box::new(AgentCap {
        program: AgentProgram::parse(program).expect("program"),
        key_env: Some(EnvName::parse("SOME_API_KEY").expect("env")),
        base_url_env: base_url_env.map(|name| EnvName::parse(name).expect("env")),
        protocols: protocols.iter().copied().collect(),
    })))
}

fn agent_need(program: &str, protocols: &[AgentProtocol], base_url: bool) -> Need {
    Need::Agent(AgentNeed {
        program: AgentProgram::parse(program).expect("program"),
        protocols: protocols.iter().copied().collect(),
        base_url,
    })
}

fn storage_need(access: Access, delta: Delta, scope: StorageScope) -> Need {
    Need::Storage(StorageNeed {
        access,
        delta,
        scope,
        quota: false,
    })
}

fn present(capability: Capability) -> Offer {
    Offer::Present(capability)
}

fn computer_use(environments: &[ComputerUseEnv]) -> Offer {
    present(Capability::ComputerUse(ComputerUseCap {
        environments: environments.iter().copied().collect(),
        batching: ComputerUseBatching::Many,
        zoom: false,
        max_image: Px(1568),
        wire: LlmWire::ChatCompletions,
    }))
}

fn computer_use_need(environments: &[ComputerUseEnv]) -> Need {
    Need::ComputerUse(ComputerUseNeed {
        environments: environments.iter().copied().collect(),
    })
}

fn google_photos() -> Offer {
    present(Capability::Photos(PhotosCap {
        library_read: LibraryRead::PickerOnly,
        upload: true,
        albums: Albums::AppCreated,
        video: true,
        delta: Delta::None,
    }))
}

#[expect(
    clippy::too_many_lines,
    reason = "one table of matching cases; each row is a short need, offer and verdict"
)]
fn cases() -> Vec<(&'static str, Need, Offer, Match)> {
    use Access::{Read, ReadWrite};
    use StorageScope::{AppFolder, Full};
    let chat_tools: &[LlmFeature] = &[LlmFeature::Chat, LlmFeature::Tools];
    vec![
        (
            "storage write+poll fits a push store",
            storage_need(ReadWrite, Delta::Poll, AppFolder),
            present(storage(ReadWrite, Delta::Push, Full)),
            Match::Fits,
        ),
        (
            "equal minimums fit",
            storage_need(ReadWrite, Delta::Poll, AppFolder),
            present(storage(ReadWrite, Delta::Poll, AppFolder)),
            Match::Fits,
        ),
        (
            "read-only store falls short on access",
            storage_need(ReadWrite, Delta::Poll, AppFolder),
            present(storage(Read, Delta::Push, Full)),
            Match::Short(Shortfall::Access),
        ),
        (
            "no-delta store falls short on delta",
            storage_need(Read, Delta::Poll, AppFolder),
            present(storage(ReadWrite, Delta::None, Full)),
            Match::Short(Shortfall::Delta),
        ),
        (
            "app folder falls short of a full-store need",
            storage_need(Read, Delta::None, Full),
            present(storage(ReadWrite, Delta::Push, AppFolder)),
            Match::Short(Shortfall::Scope),
        ),
        (
            "first shortfall wins",
            storage_need(ReadWrite, Delta::Push, Full),
            present(storage(Read, Delta::None, AppFolder)),
            Match::Short(Shortfall::Access),
        ),
        (
            "absent storage says why",
            storage_need(Read, Delta::None, AppFolder),
            Offer::Absent {
                kind: CapabilityKind::Storage,
                reason: AbsentReason::ProviderOffersNone,
            },
            Match::Absent(AbsentReason::ProviderOffersNone),
        ),
        (
            "absent of another kind is another kind",
            storage_need(Read, Delta::None, AppFolder),
            Offer::Absent {
                kind: CapabilityKind::Notes,
                reason: AbsentReason::TenantConsent,
            },
            Match::OtherKind,
        ),
        (
            "mail offer does not answer a storage need",
            storage_need(Read, Delta::None, AppFolder),
            present(mail(true)),
            Match::OtherKind,
        ),
        (
            "mail without send falls short on send",
            Need::Mail(MailNeed {
                access: Read,
                send: true,
                delta: Delta::None,
            }),
            present(mail(false)),
            Match::Short(Shortfall::Send),
        ),
        (
            "google photos cannot mirror a library",
            Need::Photos(PhotosNeed {
                library_read: LibraryRead::Full,
                upload: false,
                albums: Albums::None,
                video: false,
                delta: Delta::None,
            }),
            google_photos(),
            Match::Short(Shortfall::LibraryRead),
        ),
        (
            "google photos is an upload target",
            Need::Photos(PhotosNeed {
                library_read: LibraryRead::None,
                upload: true,
                albums: Albums::AppCreated,
                video: false,
                delta: Delta::None,
            }),
            google_photos(),
            Match::Fits,
        ),
        (
            "llm with the features and window fits",
            Need::Llm(LlmNeed {
                features: chat_tools.iter().copied().collect(),
                context: Tokens(32_000),
            }),
            present(llm(
                &[LlmFeature::Chat, LlmFeature::Tools, LlmFeature::Vision],
                128_000,
            )),
            Match::Fits,
        ),
        (
            "llm without tools falls short on features",
            Need::Llm(LlmNeed {
                features: chat_tools.iter().copied().collect(),
                context: Tokens(8_000),
            }),
            present(llm(&[LlmFeature::Chat], 128_000)),
            Match::Short(Shortfall::Features),
        ),
        (
            "llm with a small window falls short on context",
            Need::Llm(LlmNeed {
                features: std::collections::BTreeSet::new(),
                context: Tokens(32_000),
            }),
            present(llm(&[LlmFeature::Chat], 8_000)),
            Match::Short(Shortfall::Context),
        ),
        (
            "llm with an unstated window is not known to be too small",
            Need::Llm(LlmNeed {
                features: std::collections::BTreeSet::new(),
                context: Tokens(32_000),
            }),
            present(llm(&[LlmFeature::Chat], 0)),
            Match::Fits,
        ),
        (
            "the agent asked for fits the agent offered",
            agent_need("claude-code", &[], false),
            agent("claude-code", None, &[AgentProtocol::AnthropicMessages]),
            Match::Fits,
        ),
        (
            "another agent program is not the one asked for",
            agent_need("codex", &[], false),
            agent("claude-code", Some("ANTHROPIC_BASE_URL"), &[]),
            Match::Short(Shortfall::Program),
        ),
        (
            "an agent that speaks only Messages falls short of an OpenAI-compatible need",
            agent_need("claude-code", &[AgentProtocol::OpenAiCompatible], false),
            agent("claude-code", None, &[AgentProtocol::AnthropicMessages]),
            Match::Short(Shortfall::Protocols),
        ),
        (
            "an agent with no base-url variable falls short of a routed need",
            agent_need("gemini-cli", &[], true),
            agent("gemini-cli", None, &[AgentProtocol::OpenAiCompatible]),
            Match::Short(Shortfall::BaseUrl),
        ),
        (
            "an agent with a base-url variable fits a routed need",
            agent_need("codex", &[AgentProtocol::OpenAiCompatible], true),
            agent(
                "codex",
                Some("OPENAI_BASE_URL"),
                &[AgentProtocol::OpenAiCompatible],
            ),
            Match::Fits,
        ),
        (
            "an llm offer is another kind for an agent need",
            agent_need("codex", &[], false),
            present(llm(&[LlmFeature::Chat], 8_000)),
            Match::OtherKind,
        ),
        (
            "computer use on the desktop fits a desktop model",
            computer_use_need(&[ComputerUseEnv::Desktop]),
            computer_use(&[ComputerUseEnv::Desktop, ComputerUseEnv::Browser]),
            Match::Fits,
        ),
        (
            "computer use on a phone falls short on environments",
            computer_use_need(&[ComputerUseEnv::Desktop, ComputerUseEnv::Mobile]),
            computer_use(&[ComputerUseEnv::Desktop]),
            Match::Short(Shortfall::Environments),
        ),
        (
            "a computer-use need is not met by a chat model",
            computer_use_need(&[ComputerUseEnv::Desktop]),
            present(llm(&[LlmFeature::Vision], 8_000)),
            Match::OtherKind,
        ),
    ]
}

#[test]
fn matches_answers_each_need_against_each_offer() {
    for (name, need, offer, expected) in cases() {
        assert_eq!(matches(&need, &offer), expected, "{name}");
    }
}
