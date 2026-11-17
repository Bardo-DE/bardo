//! accountd's and the sheet host's bus names and object paths: the addresses an app, a shell or a
//! test needs to find the daemon or its settings module. Plain strings; the interfaces are
//! `account-dbus`'s.

/// accountd's bus name.
pub const ACCOUNTS_BUS: &str = "org.bardo.Accounts1";
/// accountd's root object (Manager, the add-account sheet).
pub const ACCOUNTS_PATH: &str = "/org/bardo/Accounts1";
/// The desktop-wide Spaces (`org.bardo.Spaces1`), served on accountd's bus name.
pub const SPACES_PATH: &str = "/org/bardo/Spaces1";
/// The identities signed in on this machine (`org.bardo.Identities1`), served on accountd's bus
/// name.
pub const IDENTITIES_PATH: &str = "/org/bardo/Identities1";
/// Who may use which account, and the relays a grant opens (`org.bardo.Grants1`), served on
/// accountd's bus name.
pub const GRANTS_PATH: &str = "/org/bardo/Grants1";
/// The person's computers on their Tailscale network (`org.bardo.Tailnet1`), served on
/// accountd's bus name.
pub const TAILNET_PATH: &str = "/org/bardo/Tailnet1";
/// The sheet host's bus name (`org.bardo.AccountsSheet1`, served by the shell or a standalone host).
pub const SHEET_BUS: &str = "org.bardo.AccountsSheet1";
/// The sheet host's object.
pub const SHEET_PATH: &str = "/org/bardo/AccountsSheet1";
/// accountd's settings module (`org.bardo.SettingsModule1`): the account, grant and client-id
/// rows the Accounts pane reads.
pub const ACCOUNTS_SETTINGS_PATH: &str = "/org/bardo/Accounts1/settings";
/// The bus name of the inference service's model service (`org.bardo.Inference1`). The accounts service keeps the name because
/// accountd's old-key import probes for it (`old_inferd_serves_keys`).
pub const INFERENCE_BUS: &str = "org.bardo.Inference1";
/// The root object of the inference service's model service; kept for the same probe.
pub const INFERENCE_PATH: &str = "/org/bardo/Inference1";
