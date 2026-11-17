//! Whether the volume service connects to a sound server, chosen once at the edge.

/// How the volume service reaches the sound server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SoundRoute {
    /// The person's own sound server (libpulse's default connection, over pipewire-pulse).
    #[default]
    Server,
    /// No sound server: the service never connects, drops every command and keeps its
    /// passive default state.
    Absent,
}
