//! The actor kind on the wire: the projection of provenance's `Actor` that
//! `bardo_desktop_v1` stamps on state changes.

use super::code::wire_enum;

wire_enum! {
    /// Who caused a change (`bardo_desktop_manager_v1.actor_kind`).
    ActorKind {
        /// The person.
        User = 0,
        /// The companion as planner, reader or worker.
        Companion = 1,
        /// The companion as a computer-use run.
        ComputerUse = 2,
        /// An MCP client.
        Mcp = 3,
        /// A command-line tool.
        Cli = 4,
        /// An app.
        App = 5,
        /// A third-party agent.
        ThirdParty = 6,
        /// The system itself.
        System = 7,
        /// Not known.
        Unknown = 8,
    }
}
