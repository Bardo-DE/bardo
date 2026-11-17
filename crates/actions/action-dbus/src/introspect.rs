//! The introspection XML of each bus, from the skeletons: the checked-in `dbus/*.xml` must
//! equal it.

use crate::{
    AgentHostSkeleton, CheckpointSkeleton, CompanionAnswerSkeleton, CompanionHelpers2Skeleton,
    CompanionHelpersSkeleton, CompanionSkeleton, CompanionTerminalsSkeleton, ConfirmSkeleton,
    ContextSkeleton, ControlSkeleton, GateSkeleton, IndexSkeleton, IntentProviderSkeleton,
    MessageSkeleton, ReachServer, ReaderSkeleton, RegistrySkeleton, RequestSkeleton, RunSkeleton,
    SearchSkeleton, SessionSkeleton,
};
use zbus::fdo;
use zbus::object_server::Interface;

/// One of the buses declared here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bus {
    /// `org.bardo.Intents1` (intentd).
    Intents,
    /// `org.bardo.IntentProvider1` (every provider).
    IntentProvider,
    /// `org.bardo.IntentReach1` (a provider whose actions' reach changes while it runs).
    IntentReach,
    /// `org.bardo.Confirm1` (the shell).
    Confirm,
    /// `org.bardo.Companion1` (companiond).
    Companion,
    /// `org.bardo.Reader1` (readerd).
    Reader,
    /// `org.bardo.AgentHost1` (bardo-agent).
    AgentHost,
}

impl Bus {
    /// Every bus.
    pub const ALL: [Bus; 7] = [
        Bus::Intents,
        Bus::IntentProvider,
        Bus::IntentReach,
        Bus::Confirm,
        Bus::Companion,
        Bus::Reader,
        Bus::AgentHost,
    ];

    /// The file under `dbus/` holding its introspection.
    #[must_use]
    pub fn file_name(self) -> &'static str {
        match self {
            Bus::Intents => "org.bardo.Intents1.xml",
            Bus::IntentProvider => "org.bardo.IntentProvider1.xml",
            Bus::IntentReach => "org.bardo.IntentReach1.xml",
            Bus::Confirm => "org.bardo.Confirm1.xml",
            Bus::Companion => "org.bardo.Companion1.xml",
            Bus::Reader => "org.bardo.Reader1.xml",
            Bus::AgentHost => "org.bardo.AgentHost1.xml",
        }
    }
}

/// The introspection document of `bus`: every interface it serves, in one `<node>`.
#[must_use]
pub fn introspection(bus: Bus) -> String {
    let reach = ReachServer::new();
    let interfaces: Vec<&dyn Interface> = match bus {
        Bus::Intents => vec![
            &RegistrySkeleton,
            &IndexSkeleton,
            &SearchSkeleton,
            &RunSkeleton,
            &ContextSkeleton,
            &SessionSkeleton,
            &CheckpointSkeleton,
            &MessageSkeleton,
            &GateSkeleton,
            &ControlSkeleton,
            &RequestSkeleton,
        ],
        Bus::IntentProvider => vec![&IntentProviderSkeleton],
        Bus::IntentReach => vec![&reach],
        Bus::Confirm => vec![&ConfirmSkeleton],
        Bus::Companion => vec![
            &CompanionSkeleton,
            &CompanionAnswerSkeleton,
            &CompanionHelpersSkeleton,
            &CompanionHelpers2Skeleton,
            &CompanionTerminalsSkeleton,
        ],
        Bus::Reader => vec![&ReaderSkeleton],
        Bus::AgentHost => vec![&AgentHostSkeleton],
    };
    let mut xml = String::from(
        "<!DOCTYPE node PUBLIC \"-//freedesktop//DTD D-BUS Object Introspection 1.0//EN\"\n \"http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd\">\n<node>\n",
    );
    for interface in interfaces {
        interface.introspect_to_writer(&mut xml, 1);
    }
    xml.push_str("</node>\n");
    xml
}

/// The answer of every skeleton method: the interface is frozen, its behaviour not built.
pub(crate) fn frozen() -> fdo::Error {
    fdo::Error::NotSupported("docket: frozen interface, not implemented".into())
}
