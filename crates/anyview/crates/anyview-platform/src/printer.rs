//! Printing a PDF through the desktop's own dialog.

use crate::error::PlatformError;
use ui_core::word::Word;
use std::future::Future;

/// How a print request ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum PrintOutcome {
    /// The dialog was accepted and the job handed over.
    Printed,
    /// The person closed the dialog.
    Cancelled,
    /// There is no print dialog to show; the caller opens the PDF in a viewer instead.
    NoDialog,
}

/// Print a PDF.
pub trait Printer {
    /// Show the print dialog and, if the person accepts it, print `pdf`; `title` is what the job
    /// shows in the queue.
    fn print(
        &self,
        pdf: &[u8],
        title: &str,
    ) -> impl Future<Output = Result<PrintOutcome, PlatformError>> + Send;

    /// Whether this implementation has the service behind it. An implementation that answers
    /// "not available" to every request says `false`, so the views never offer what it cannot do.
    fn present(&self) -> bool {
        true
    }
}
