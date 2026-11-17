//! Wi-Fi: NetworkManager's wireless radio, its access points and its saved networks
//! (design/20 §1.5: the control center's Wi-Fi module and its network list).
//!
//! A service of its own beside [`shell_model::network::NetworkService`] rather than more fields on `Network`: the
//! bar draws `Network` and would repaint on every scan otherwise, and the two share nothing but the
//! bus. Same proxies as the rest of the network (bardo-shell's own, in `nm`).

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::service::ServiceIo;
use shell_model::wifi::WifiService;

impl Run for WifiService {
    type Kind = Self;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        backend::run(env.buses.system.clone(), io.state, io.commands)
    }
}
