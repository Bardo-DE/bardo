//! Work that answers later, for the seams whose methods are async: a trait object cannot be.

use std::pin::Pin;

/// Work in flight that ends in a `T`.
pub type Work<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
