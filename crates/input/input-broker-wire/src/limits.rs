//! The caps the datagrams themselves carry: the protocol version, the largest datagram and the
//! longest lists a request may hold. A peer that exceeds one is refused, never allowed to make the
//! reader store more. The caps on connections, rates and devices are policy: they live in
//! `input-broker-core`.

/// The first byte of every datagram. A change of meaning bumps it; the service refuses others.
pub const PROTOCOL_VERSION: u8 = 1;

/// The largest datagram either side sends or accepts: an `Events` of 64 events is 1539 bytes.
pub const MAX_DATAGRAM: usize = 2048;

/// Events in one datagram: a touchpad frame is 8 to 12 events, so 64 holds several frames.
pub const MAX_EVENTS_PER_DATAGRAM: usize = 64;

/// Feedback events in one datagram: a keyboard has three lights.
pub const MAX_FEEDBACK_EVENTS: usize = 8;

/// Key codes the mouse gestures daemon's keyboard may ask for: its gesture table has a few dozen at most.
pub const MAX_KEY_CODES: usize = 64;

/// Buttons on a pointer: the mouse block of `linux/input-event-codes.h` is 16 wide.
pub const MAX_POINTER_BUTTONS: usize = 16;

/// Relative axes on a pointer: the kernel defines 16 and a mouse uses a handful.
pub const MAX_POINTER_RELS: usize = 8;

/// Absolute axes on a pointer: a multitouch pad has about a dozen.
pub const MAX_POINTER_ABS: usize = 16;

/// Properties on a pointer: a pad is a pointer and perhaps a button pad.
pub const MAX_POINTER_PROPS: usize = 8;

/// Bytes of a pointer's label on the wire. The broker cuts it further so the label, a space and the
/// pointer suffix fit a uinput name (80 bytes with its terminator).
pub const MAX_LABEL_BYTES: usize = 62;
