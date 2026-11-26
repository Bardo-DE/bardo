//! Bytes of a client's shared-memory pool, handed to a closure.
//!
//! smithay lends a pool as a pointer and a length for the length of one call. The client may map
//! the same memory and write it at any time, which is the nature of shared memory.
#![expect(
    unsafe_code,
    reason = "smithay lends a shm pool as a raw pointer and length; each block carries its SAFETY comment"
)]

use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::wayland::shm::{
    BufferAccessError, BufferData, with_buffer_contents, with_buffer_contents_mut,
};

/// Runs `write` over the bytes of `buffer`'s shm pool, with the buffer's layout.
///
/// The slice lasts for the call. Nothing else in this process touches the pool meanwhile; the
/// client may, and the bytes it sees may be half written.
///
/// # Errors
///
/// Returns a [`BufferAccessError`] when the buffer is not a shared-memory buffer or its pool
/// cannot be mapped.
pub fn with_shm_pool_mut<T>(
    buffer: &WlBuffer,
    write: impl FnOnce(&mut [u8], BufferData) -> T,
) -> Result<T, BufferAccessError> {
    with_buffer_contents_mut(buffer, |pointer, length, data| {
        // SAFETY: smithay lends `length` bytes at `pointer` for the length of this call and
        // nothing else in this process touches the pool meanwhile; the client may map it too,
        // which is the nature of shared memory (smithay's own docs say the same).
        let pool = unsafe { std::slice::from_raw_parts_mut(pointer, length) };
        write(pool, data)
    })
}

/// Runs `read` over a copy of the bytes of `buffer`'s shm pool that the buffer covers.
///
/// The copy has the buffer's layout (its offset is 0 in the copy): the read-only twin of
/// [`with_shm_pool_mut`], for a buffer a client drew itself.
///
/// The client may write the pool while it is read, so the bytes are copied out once and nothing
/// holds a slice over memory another process can change. A write during the copy makes a torn
/// copy, a glitch and not a fault. A buffer whose layout reaches past its pool is cut to the pool.
///
/// # Errors
///
/// Returns a [`BufferAccessError`] when the buffer is not a shared-memory buffer or its pool
/// cannot be mapped.
pub fn with_shm_pool<T>(
    buffer: &WlBuffer,
    read: impl FnOnce(&[u8], BufferData) -> T,
) -> Result<T, BufferAccessError> {
    with_buffer_contents(buffer, |pointer, length, data| {
        let start = usize::try_from(data.offset).unwrap_or(0).min(length);
        let rows = usize::try_from(data.stride)
            .unwrap_or(0)
            .saturating_mul(usize::try_from(data.height).unwrap_or(0));
        let end = start.saturating_add(rows).min(length);
        let mut copy = vec![0_u8; end - start];
        // SAFETY: smithay lends `length` bytes at `pointer` for the length of this call, and
        // `start <= end <= length`, so the `end - start` bytes read lie inside them; `copy` is a
        // fresh allocation of exactly that size, so the two cannot overlap.
        unsafe { std::ptr::copy_nonoverlapping(pointer.add(start), copy.as_mut_ptr(), copy.len()) };
        read(&copy, BufferData { offset: 0, ..data })
    })
}
