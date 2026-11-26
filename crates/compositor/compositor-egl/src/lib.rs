//! The compositor's unsafe seam over smithay. Everything here is a safe function; the `unsafe`
//! inside is the whole of what the compositor takes, and each block says why it is sound.
//!
//! Two things live here:
//!
//! - [`gles_on`] and [`surfaceless_gles`] make a GLES renderer on an EGL display. smithay marks
//!   `EGLDisplay::new` and `GlesRenderer::new` as `unsafe fn`; both calls are made here, once.
//! - [`with_shm_pool_mut`] and [`with_shm_pool`] give a client's shared-memory buffer to a closure
//!   as bytes. smithay lends the pool as a pointer and a length; the slices and the copy are made
//!   here.
//!
//! Every other GL pass in the compositor goes through smithay's safe shader API.

mod gles;
mod shm;

pub use gles::{EglError, gles_on, surfaceless_gles};
pub use shm::{with_shm_pool, with_shm_pool_mut};
