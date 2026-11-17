//! The sites an agent on the `allowlist` network may reach: the lists written in `agents.toml`
//! (`allow`, `deny`, `ports`), checked when the file is read. A site that is not a host or a
//! pattern refuses the whole file and names the program and the entry; an allow list with
//! nothing in it refuses too, because it would reach nothing and is surely a mistake.

use crate::config::ConfigError;
use sandbox::egress::{DEFAULT_PORTS, EgressPolicy, Host, HostPattern, Ports};

/// The checked lists of one program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reach {
    policy: EgressPolicy,
    sites: Vec<String>,
}

/// One entry of a list, parsed, or the fault that names it.
fn pattern(program: &str, list: &'static str, entry: &str) -> Result<HostPattern, ConfigError> {
    HostPattern::parse(entry).map_err(|why| ConfigError::Site {
        program: program.to_owned(),
        list,
        entry: entry.to_owned(),
        why: Box::new(why),
    })
}

fn patterns(
    program: &str,
    list: &'static str,
    entries: &[String],
) -> Result<Vec<HostPattern>, ConfigError> {
    entries.iter().map(|e| pattern(program, list, e)).collect()
}

/// An allowed entry as a person reads it.
fn shown(pattern: &HostPattern) -> String {
    match pattern {
        HostPattern::Exact(name) => name.clone(),
        HostPattern::Subdomains(domain) => format!("*.{domain}"),
        HostPattern::Addr(addr) => Host::Addr(*addr).to_string(),
    }
}

fn fault(program: &str, why: &str) -> ConfigError {
    ConfigError::Entry {
        program: program.to_owned(),
        why: why.to_owned(),
    }
}

/// [`fault`] for a value that was refused: `why`, then what refused it.
fn fault_value(program: &str, why: &str, cause: &impl std::fmt::Display) -> ConfigError {
    fault(program, &format!("{why}: {cause}"))
}

impl Reach {
    /// The lists of `program`. `extra_ports` are reachable besides 443 and 80.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] when an `allow` or `deny` entry is not a usable site, an extra
    /// port is not 1 to 65535, or the allow list is empty.
    pub fn new(
        program: &str,
        allow: &[String],
        deny: &[String],
        extra_ports: &[u16],
    ) -> Result<Self, ConfigError> {
        let allowed = patterns(program, "allow", allow)?;
        let denied = patterns(program, "deny", deny)?;
        let sites = allowed.iter().map(shown).collect();
        let ports = Ports::new(DEFAULT_PORTS.iter().chain(extra_ports).copied().collect())
            .map_err(|e| fault_value(program, "a port is 1 to 65535", &e))?;
        let policy = EgressPolicy::new(allowed, denied, ports).map_err(|e| {
            fault_value(
                program,
                "network allowlist needs at least one entry in allow",
                &e,
            )
        })?;
        Ok(Self { policy, sites })
    }

    /// The policy the filter enforces.
    #[must_use]
    pub fn policy(&self) -> &EgressPolicy {
        &self.policy
    }

    /// The allowed sites, as written (lower-cased), for showing the person.
    #[must_use]
    pub fn sites(&self) -> &[String] {
        &self.sites
    }
}
