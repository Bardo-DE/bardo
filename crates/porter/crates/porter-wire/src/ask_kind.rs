/// What accountd asks of an agent launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskKind {
    /// Have the agent sign itself in.
    Login,
    /// Have the agent sign itself out.
    Logout,
}
