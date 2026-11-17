//! The network: NetworkManager's primary connection over the system bus
//! (the `networkmanager-proxy` proxies).

mod backend;

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::Run;
use shell_model::network::NetworkService;
use shell_model::service::ServiceIo;

impl Run for NetworkService {
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        backend::run(env.buses.system.clone(), io.state, io.commands)
    }
}
