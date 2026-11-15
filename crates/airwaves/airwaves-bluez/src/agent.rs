//! `org.bluez.AgentManager1`, and what a program's own `org.bluez.Agent1` says and refuses with.

use zbus::proxy;
use zbus::zvariant::ObjectPath;

/// The agent manager, at `/org/bluez`.
#[proxy(
    interface = "org.bluez.AgentManager1",
    default_service = "org.bluez",
    default_path = "/org/bluez"
)]
pub trait AgentManager1 {
    /// Registers the agent the program serves at `agent`; `capability` is
    /// [`Capability::to_wire`].
    fn register_agent(&self, agent: &ObjectPath<'_>, capability: &str) -> zbus::Result<()>;

    /// Unregisters it.
    fn unregister_agent(&self, agent: &ObjectPath<'_>) -> zbus::Result<()>;
}

/// What an agent can do for a pairing, which decides how BlueZ pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Show a number only.
    DisplayOnly,
    /// Show a number and take a yes or a no.
    DisplayYesNo,
    /// Take typed digits only.
    KeyboardOnly,
    /// Neither show nor take anything: pairing goes through without asking.
    NoInputNoOutput,
    /// Show a number, take a yes or a no, and take typed digits.
    KeyboardDisplay,
}

impl Capability {
    /// The string `RegisterAgent` takes.
    pub fn to_wire(self) -> &'static str {
        match self {
            Self::DisplayOnly => "DisplayOnly",
            Self::DisplayYesNo => "DisplayYesNo",
            Self::KeyboardOnly => "KeyboardOnly",
            Self::NoInputNoOutput => "NoInputNoOutput",
            Self::KeyboardDisplay => "KeyboardDisplay",
        }
    }
}

/// The errors an agent answers BlueZ with. BlueZ reads `Rejected` as the person saying no and
/// `Canceled` as the question going away.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.bluez.Error")]
pub enum AgentError {
    /// The bus itself failed.
    #[zbus(error)]
    ZBus(zbus::Error),
    /// The person said no.
    Rejected(String),
    /// The question was withdrawn.
    Canceled(String),
}
