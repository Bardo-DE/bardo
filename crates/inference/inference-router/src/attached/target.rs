//! Where an attached engine is and how a request to it is signed: the endpoint of one connect.

use super::config::{Attached, Reach, Site};
use super::key::{KeyFile, KeyFileProblem};
use base_secret::SecretText;
use inference_core::ComputerName;
use model_http::{AuthHeader, HostName, HttpEndpoint, HttpTarget, Proxy, Timeouts, UrlPath};
use std::path::PathBuf;

/// An attached engine as a connection is made to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// How it is reached.
    pub reach: Reach,
    /// Its bearer token's file, when it wants one.
    pub key: Option<KeyFile>,
    /// Where the data goes.
    pub place: Site,
    /// The machine it runs on, when the entry names one.
    pub computer: Option<ComputerName>,
}

impl Target {
    /// The target an entry names.
    pub fn of(attached: &Attached) -> Self {
        Self {
            reach: attached.reach.clone(),
            key: attached.key_file.clone().map(KeyFile::at),
            place: attached.place,
            computer: attached.computer.clone(),
        }
    }

    /// The socket, when the engine is reached over one.
    #[must_use]
    pub fn socket(&self) -> Option<&PathBuf> {
        match &self.reach {
            Reach::Socket(path) | Reach::Tailnet { socket: path, .. } => Some(path),
            Reach::Loopback { .. } => None,
        }
    }

    /// Whether the engine is a computer on the Tailscale network, reached through a relay that
    /// asks Tailscale and the computer before it answers: slower to answer than a socket the
    /// person's own tunnel holds open.
    #[must_use]
    pub fn is_relayed(&self) -> bool {
        self.reach.is_tailnet()
    }

    /// The endpoint of one connect: the key file is read now through `read_key` (so a token that
    /// was rotated is the one sent; reading a file is the caller's, `inference_fs::read_key`), and
    /// a file that is refused is why there is no endpoint.
    ///
    /// # Errors
    /// `KeyFileProblem` when `read_key` refuses the key file.
    pub fn endpoint(
        &self,
        base: &str,
        timeouts: Timeouts,
        read_key: impl FnOnce(&KeyFile) -> Result<SecretText, KeyFileProblem>,
    ) -> Result<HttpEndpoint, KeyFileProblem> {
        let auth = match (&self.key, &self.reach) {
            // Plain http on this machine's loopback: the key goes only because it is opted in, and
            // the client still refuses any host that is not loopback.
            (Some(key), Reach::Loopback { .. }) => AuthHeader::LoopbackBearer(read_key(key)?),
            (Some(key), _) => AuthHeader::Bearer(read_key(key)?),
            (None, _) => AuthHeader::None,
        };
        Ok(self.endpoint_with(auth, base, timeouts))
    }

    /// The endpoint with no token: what a turn sends when the key file is refused (the engine
    /// answers 401, and no token has gone anywhere).
    #[must_use]
    pub fn unsigned(&self, base: &str, timeouts: Timeouts) -> HttpEndpoint {
        self.endpoint_with(AuthHeader::None, base, timeouts)
    }

    fn endpoint_with(&self, auth: AuthHeader, base: &str, timeouts: Timeouts) -> HttpEndpoint {
        let target = match &self.reach {
            Reach::Socket(path) | Reach::Tailnet { socket: path, .. } => {
                HttpTarget::Unix(path.clone())
            }
            Reach::Loopback { host, port } => HttpTarget::Tcp {
                host: HostName(host.to_string()),
                port: *port,
            },
        };
        HttpEndpoint {
            target,
            proxy: Proxy::Direct,
            base: UrlPath(base.to_owned()),
            auth,
            headers: Vec::new(),
            timeouts,
        }
    }
}
