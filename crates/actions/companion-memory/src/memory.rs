//! The router's memory seam over memoryd.

use memory_client::{ClientError, Memory, Transport};
use memory_core::{HistoryFate, MemoryFate, MemoryReply, MemoryRequest, Removal};
use action_core::{LinkFault, MemoryLink};
use provenance::SpaceId;

/// Memory, as the router asks it (`Caller::Router`).
#[derive(Debug, Clone)]
pub struct AlmanacMemory<T: Transport> {
    memory: Memory<T>,
}

impl<T: Transport> AlmanacMemory<T> {
    /// Asks memoryd over `transport`.
    pub fn over(transport: T) -> Self {
        Self {
            memory: Memory::over(transport),
        }
    }
}

impl<T: Transport> MemoryLink for AlmanacMemory<T> {
    async fn ask(&self, request: MemoryRequest) -> Result<MemoryReply, LinkFault> {
        match self.memory.ask(request).await {
            Ok(reply) => Ok(reply),
            // A refusal is memoryd's answer, not a failure of the link: the router decides what
            // it means.
            Err(ClientError::Refused(refusal)) => Ok(MemoryReply::Refused(refusal)),
            Err(ClientError::Transport(_)) => Err(LinkFault::Unavailable),
            Err(ClientError::Unexpected) => Err(LinkFault::Malformed),
        }
    }

    async fn erase_space(&self, space: &SpaceId) -> bool {
        matches!(
            self.ask(MemoryRequest::RemoveSpace(
                space.clone(),
                // The person chose to delete the memories; the history is a separate choice,
                // made in the removal sheet, and is kept here.
                Removal {
                    memories: MemoryFate::Delete,
                    history: HistoryFate::Keep,
                },
            ))
            .await,
            Ok(MemoryReply::Relocated(_))
        )
    }
}
