//! The capabilities and the bus names they mean.

/// Which bus a service lives on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bus {
    /// The per-login session bus.
    Session,
    /// The system bus.
    System,
}

/// A well-known D-Bus name on one bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Service {
    /// The bus that carries it.
    pub bus: Bus,
    /// The well-known name.
    pub name: &'static str,
}

const fn session(name: &'static str) -> Service {
    Service {
        bus: Bus::Session,
        name,
    }
}

/// One extra the desktop adds to an app's portable core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// intentd: the registry of what apps can do (`bardo do`, the companion's actions).
    Intents,
    /// accountd: the session's one account store and consent sheet.
    Accounts,
    /// memoryd: one memory for every app.
    Memory,
    /// companiond: the one companion across all apps.
    Companion,
    /// The desktop broadcasts its appearance (accent, Space tint) beyond the portal.
    Appearance,
    /// Compositor materials: blur behind a surface.
    Materials,
    /// The shell's Quick Look peek of a file.
    Peek,
    /// The shell's share sheet.
    Share,
    /// `PackageKit` answers, so a missing helper can be installed on the spot.
    Helpers,
    /// The shell's per-output work area and scale (`org.bardo.Outputs1`), so a window can be
    /// sized and placed against the usable screen. The portable fallback is the window system's
    /// own output size ([`crate::Outputs`] is empty).
    Outputs,
}

impl Capability {
    /// Every capability, in declaration order.
    pub const ALL: [Capability; 10] = [
        Capability::Intents,
        Capability::Accounts,
        Capability::Memory,
        Capability::Companion,
        Capability::Appearance,
        Capability::Materials,
        Capability::Peek,
        Capability::Share,
        Capability::Helpers,
        Capability::Outputs,
    ];

    /// The D-Bus name that means this capability, or `None` where no service name is settled
    /// (such a capability is always `Absent` until one is).
    #[must_use]
    #[expect(
        clippy::match_same_arms,
        reason = "each capability without a service name has its own reason, kept beside it"
    )]
    pub const fn service(self) -> Option<Service> {
        match self {
            // design/33-AGENT.md section 5 (intentd, docket repo).
            Capability::Intents => Some(session("org.bardo.Intents1")),
            // accountd, porter repo (porter/crates/accountd/src/lib.rs:2); also named by
            // ui-settings' live client docs.
            Capability::Accounts => Some(session("org.bardo.Accounts1")),
            // memoryd, design/33-AGENT.md section 5 (almanac repo).
            Capability::Memory => Some(session("org.bardo.Memory1")),
            // companiond, design/33-AGENT.md section 5 (docket repo).
            Capability::Companion => Some(session("org.bardo.Companion1")),
            // bardo-shell's `work_areas` service (bardo-shell/crates/shell-services/src/work_areas).
            Capability::Outputs => Some(session("org.bardo.Outputs1")),
            Capability::Helpers => Some(Service {
                bus: Bus::System,
                name: "org.freedesktop.PackageKit",
            }),
            // TODO: no service name exists yet. Appearance today is the freedesktop settings
            // portal, which other desktops also serve, so it cannot tell ours apart.
            Capability::Appearance => None,
            // TODO: blur is a Wayland protocol (ext-background-effect-v1), not a bus service.
            Capability::Materials => None,
            // TODO: anyview serves `org.bardo.Anyview1.Peek`, but the shell's peek endpoint has
            // no name in bardo-shell or layer-shell-host yet.
            Capability::Peek => None,
            // TODO: no share sheet service is specified in any repo yet.
            Capability::Share => None,
        }
    }
}
