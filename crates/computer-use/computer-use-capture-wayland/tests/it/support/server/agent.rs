//! The agent side of the fake compositor: `bardo_agent_manager_v1` grants or refuses a lease,
//! the lease hands out a capture source of its window and records the mask hints it is sent.

use super::Compositor;
use desktop_protocols::server::bardo_agent_v1::bardo_agent_lease_v1::{
    self, BardoAgentLeaseV1, RevokeReason,
};
use desktop_protocols::server::bardo_agent_v1::bardo_agent_manager_v1::{
    self, BardoAgentManagerV1,
};
use wayland_server::{Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource};

/// What the compositor answers to `create_lease`.
#[derive(Debug, Clone, Copy)]
pub enum Stance {
    /// `granted`.
    Grant,
    /// `revoked` straight away, for this reason.
    Refuse(RevokeReason),
}

impl GlobalDispatch<BardoAgentManagerV1, ()> for Compositor {
    fn bind(
        _: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        resource: New<BardoAgentManagerV1>,
        _: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        init.init(resource, ());
    }
}

impl Dispatch<BardoAgentManagerV1, ()> for Compositor {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &BardoAgentManagerV1,
        request: bardo_agent_manager_v1::Request,
        _: &(),
        _: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        if let bardo_agent_manager_v1::Request::CreateLease { id, toplevel, .. } = request {
            let top = toplevel.data::<usize>().copied().unwrap_or(usize::MAX);
            let lease = init.init(id, top);
            match state.stance {
                Stance::Grant => lease.granted(),
                Stance::Refuse(reason) => lease.revoked(reason),
            }
        }
    }
}

impl Dispatch<BardoAgentLeaseV1, usize> for Compositor {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &BardoAgentLeaseV1,
        request: bardo_agent_lease_v1::Request,
        top: &usize,
        _: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        use bardo_agent_lease_v1::Request;
        match request {
            Request::GetCaptureSource { source } => {
                init.init(source, *top);
            }
            Request::SetMaskHints { hints } => state.hints.push(hints),
            _ => {}
        }
    }
}
