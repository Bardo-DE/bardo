//! The service's own caps, each with the reason it has this number. A peer that exceeds one is
//! refused or cut, never allowed to make the service store or wait for more. The caps the
//! datagrams carry (sizes, list lengths) are `input_broker_wire::limits`.

/// Keys held at once on the mouse gestures daemon's keyboard: a combination is three or four.
pub const MAX_KEYS_DOWN: usize = 8;

/// Devices one connection may have at once.
pub const MAX_DEVICES_PER_CONNECTION: usize = 4;

/// Devices one connection may ask for in its life: ids are one byte and are never reused.
pub const MAX_CREATES_PER_CONNECTION: u8 = 64;

/// Handed devices one connection may ask for: the service cannot see them close, so it counts them.
pub const MAX_HANDED_PER_CONNECTION: u8 = 16;

/// Connections at once from callers that are not the mouse gestures daemon or the key remap daemon:
///
/// the units run one the mouse gestures daemon and one the key remap daemon, so nothing else has a
/// use for more.
pub const MAX_CONNECTIONS: usize = 16;

/// Connections at once from one user that is not shown to be the mouse gestures daemon or the key remap daemon: one local account
/// cannot take the slots of the others.
pub const MAX_CONNECTIONS_PER_USER: usize = 4;

/// Connections reserved for each of the mouse gestures daemon and the key remap daemon once they
/// are proven:
///
/// a client reconnects after a cut and may briefly hold the old and the new, so more than one;
/// outside every other cap.
pub const MAX_RESERVED_PER_CLIENT: usize = 4;

/// Connections being identified at once from one user (NEW-FA-3).
///
/// Identification reads the peer's `/proc` entries and can wait on a program the peer controls, so
/// each one has a thread of its own, and one account may hold two (a client that reconnects while
/// it still has the old connection).
pub const MAX_PENDING_PER_USER: usize = 2;

/// Connections being identified at once in all: each is a thread beyond the 24 that can hold a slot,
/// so this is what is left of the unit's thread limit after the accept loop and those 24.
pub const MAX_PENDING_IDENTIFICATIONS: usize = 6;

/// Threads the unit allows, `TasksMax=` in `dist/bardo-input-broker.service`: keep the two in step.
pub const SERVICE_TASKS_MAX: usize = 32;

// The accept loop, every slot (strangers and both reserves) and every pending identification each
// have a thread; together they must fit the unit's limit.
const _: () = assert!(
    1 + MAX_CONNECTIONS + 2 * MAX_RESERVED_PER_CLIENT + MAX_PENDING_IDENTIFICATIONS
        <= SERVICE_TASKS_MAX
);

/// `Events` datagrams per second per device: a touchpad frame at 120 Hz is one datagram.
pub const EVENT_DATAGRAMS_PER_SECOND: u64 = 120;

/// Datagrams that may arrive in a burst before the rate applies.
pub const EVENT_BURST: u64 = 30;

/// A connection that keeps exceeding the rate for this long is cut.
pub const FLOOD_CUT_MS: u64 = 3_000;

/// A pause in the excess this long ends one flood and starts another count.
pub const FLOOD_GAP_MS: u64 = 1_000;

/// Events refused by a ceiling over a connection's life before it is cut: a bug sends a few, a
/// hostile peer sends them all.
pub const MAX_DENIED_EVENTS: u32 = 256;

/// A new connection of a proven the mouse gestures daemon or the key remap daemon that has made no device in this long is closed.
pub const HANDSHAKE_TIMEOUT_MS: u64 = 1_000;

/// Any other connection that has made no device in this long is closed: it has no business waiting.
pub const STRANGER_HANDSHAKE_TIMEOUT_MS: u64 = 200;

/// The largest wheel step in one event: a fast flick is a few hundred hi-res units.
pub const MAX_WHEEL_MAGNITUDE: i32 = 1_200;
