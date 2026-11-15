//! The fake sill: `FakeShellCalls` records the D-Bus calls bindings make (A4), so a test asserts
//! that a binding called the method within one frame. A clone shares the record.

use casement_core::input::DbusCall;
use casement_ipc::shell_calls::{FailedCall, ShellCallError, ShellCalls};
use std::sync::{Arc, Mutex, PoisonError};

/// A recorder standing in for the session bus.
#[derive(Debug, Clone, Default)]
pub struct FakeShellCalls {
    calls: Arc<Mutex<Vec<DbusCall>>>,
}

impl FakeShellCalls {
    /// A recorder with no calls.
    pub fn new() -> FakeShellCalls {
        FakeShellCalls::default()
    }

    /// The calls made so far, in order.
    pub fn calls(&self) -> Vec<DbusCall> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl ShellCalls for FakeShellCalls {
    fn call(&mut self, call: &DbusCall) -> Result<(), ShellCallError> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call.clone());
        Ok(())
    }

    fn failures(&mut self) -> Vec<FailedCall> {
        Vec::new()
    }
}
