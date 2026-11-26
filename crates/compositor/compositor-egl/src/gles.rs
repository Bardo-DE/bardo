//! A GLES renderer on an EGL display: the two constructors smithay marks `unsafe fn`.
#![expect(
    unsafe_code,
    reason = "smithay's EGLDisplay::new and GlesRenderer::new are unsafe fn; each call carries its SAFETY comment"
)]

use smithay::backend::egl::native::{EGLNativeDisplay, EGLSurfacelessDisplay};
use smithay::backend::egl::{self, EGLContext, EGLDisplay};
use smithay::backend::renderer::gles::{GlesError, GlesRenderer};

/// Why a GLES renderer could not be made.
#[derive(Debug, thiserror::Error)]
pub enum EglError {
    /// No EGL display could be made on the native display.
    #[error("EGL display: {0}")]
    Display(#[source] egl::Error),
    /// The display gave no GLES context.
    #[error("EGL context: {0}")]
    Context(#[source] egl::Error),
    /// smithay could not set up its GLES renderer on the context.
    #[error("GLES renderer: {0}")]
    Renderer(#[source] GlesError),
}

/// A GLES renderer on the EGL surfaceless platform: no DRM node, no window system.
///
/// In the jail this is Mesa's llvmpipe (the jail pins the Mesa EGL vendor), the headless backend's
/// `SoftRenderer::Llvmpipe` and the tests' GLES renderer.
///
/// # Errors
///
/// Returns an [`EglError`] when no EGL display or GLES context can be made.
pub fn surfaceless_gles() -> Result<GlesRenderer, EglError> {
    gles_on(EGLSurfacelessDisplay)
}

/// A GLES renderer on the EGL display of `native` (a GBM device of a KMS node: the tty backend's
/// renderer for the outputs that device scans out).
///
/// The program must make every EGL display through this function (or [`surfaceless_gles`]): smithay
/// tracks the displays it made, and a display made another way with the same parameters could be
/// terminated behind its back.
///
/// # Errors
///
/// Returns an [`EglError`] when no EGL display or GLES context can be made on `native`.
pub fn gles_on<N: EGLNativeDisplay + 'static>(native: N) -> Result<GlesRenderer, EglError> {
    // SAFETY: `EGLDisplay::new` is unsafe because a display obtained elsewhere through
    // `eglGetPlatformDisplay` with the same parameters could be terminated behind smithay's
    // back. Nothing in the compositor creates an EGL display except this function, and the
    // compositor links no other EGL user, so smithay's instance tracking holds.
    let display = unsafe { EGLDisplay::new(native) }.map_err(EglError::Display)?;
    let context = EGLContext::new(&display).map_err(EglError::Context)?;
    // SAFETY: `GlesRenderer::new` is undefined behaviour if the context is current on another
    // thread. The context was created on this thread a statement ago and has not been shared or
    // made current anywhere; the renderer takes ownership of it.
    unsafe { GlesRenderer::new(context) }.map_err(EglError::Renderer)
}
