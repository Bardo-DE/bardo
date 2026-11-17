//! Who is calling memoryd. Derived by the transport, never sent in a request.

use caller_role_types::CallerRole;
use memory_core::{AppId, AppName, Isolation};

/// Who is calling: the input of the authorisation matrix ([`crate::allowed`]).
///
/// It is not wire: no request, reply or stored file carries one. The daemon maps the D-Bus sender
/// to its app and its shared role ([`CallerRole`], porter R12: unsandboxed callers are advisory).
/// Of the roles, four stand for the fixed callers of memoryd, and every other is a plain app for
/// its own things:
///
/// - `Agent`: docket's intentd, the router;
/// - `Cua`: cuad, for its run records;
/// - `SheetHost`: the shell's own UI (the shell), the person;
/// - `Settings`: the System Settings app (the Settings app), the person, saving a setting. Not an app with
///   memory: it may only confirm a loosened memory setting and read the ones that wait for it.
///
/// A library user picks the caller when it builds its transport (`memory-client`'s `InProcess`).
///
/// The companion has no role of its own here: it reads and proposes through the action router
/// (`org.bardo.Memory` is a built-in provider of intentd), which calls as the router acting for
/// `Actor::Companion` with the Space taken from the invocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Caller {
    role: CallerRole,
    app: AppId,
}

impl Caller {
    /// The caller that is `app` in `role`. A role that is not one of the four above makes the
    /// caller a plain app; the daemon gives the four only to the app that is entitled to each.
    #[must_use]
    pub fn new(role: CallerRole, app: AppId) -> Self {
        let role = match role {
            CallerRole::Agent | CallerRole::Cua | CallerRole::SheetHost | CallerRole::Settings => {
                role
            }
            _ => CallerRole::App,
        };
        Self { role, app }
    }

    /// A plain app, for its own things.
    #[must_use]
    pub fn app(app: AppId) -> Self {
        Self::new(CallerRole::App, app)
    }

    /// The action router (intentd).
    #[must_use]
    pub fn router() -> Self {
        Self::fixed(CallerRole::Agent, "org.bardo.Intents")
    }

    /// cuad.
    #[must_use]
    pub fn cuad() -> Self {
        Self::fixed(CallerRole::Cua, "org.bardo.ComputerUse")
    }

    /// The shell's own UI (the shell): the person.
    #[must_use]
    pub fn shell_ui() -> Self {
        Self::fixed(CallerRole::SheetHost, "org.bardo.Shell")
    }

    /// The System Settings app (the Settings app).
    #[must_use]
    pub fn settings() -> Self {
        Self::fixed(CallerRole::Settings, "org.bardo.Settings")
    }

    fn fixed(role: CallerRole, name: &str) -> Self {
        let name = AppName::parse(name).expect("a fixed caller's app name is well formed");
        Self::new(
            role,
            AppId {
                name,
                isolation: Isolation::Unsandboxed,
            },
        )
    }

    /// The shared role, as the daemon mapped it.
    #[must_use]
    pub fn role(&self) -> CallerRole {
        self.role
    }

    /// The app behind the connection.
    #[must_use]
    pub fn app_id(&self) -> &AppId {
        &self.app
    }
}
