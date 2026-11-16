//! The header and its canonical bytes: defined in `memory-store`, re-exported here.

pub use memory_store::{
    GENESIS_CONTEXT, HEADER_MAGIC, Header, NewHeader, body_digest, genesis_link, header_bytes, link,
};
