//! `org.bardo.AccountsSheet1`, as the shell exports it. The object only checks who is calling and
//! hands the call to the loop, which decides and answers.

use std::sync::Arc;

use account_dbus::accounts::AccountsSheetServer;
use account_sheet::SheetView;
use dbus_kit::Call as BusCall;
use shell_model::accounts::sheet::SheetHandle;
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use zbus::fdo;

use super::callers::{Caller, from_service};
use crate::callers::Peers;

/// The largest view taken, in bytes: a provider list or a review is a few kilobytes.
const MAX_VIEW: usize = 64 * 1024;

/// What the loop is asked.
#[derive(Debug)]
pub(super) enum Call {
    Open {
        handle: SheetHandle,
        view: Box<SheetView>,
        opener: Caller,
        answer: oneshot::Sender<fdo::Result<()>>,
    },
    Update {
        handle: SheetHandle,
        view: Box<SheetView>,
        from: Caller,
        answer: oneshot::Sender<fdo::Result<()>>,
    },
    Close {
        handle: SheetHandle,
        from: Caller,
        answer: oneshot::Sender<fdo::Result<()>>,
    },
}

/// The sheet object.
pub(super) struct SheetObject {
    pub calls: UnboundedSender<Call>,
    /// Tells the callers on the sheet's connection apart.
    pub peers: Arc<Peers>,
}

fn handle_of(text: &str) -> fdo::Result<SheetHandle> {
    SheetHandle::parse(text).ok_or_else(|| fdo::Error::InvalidArgs("not a sheet handle".to_owned()))
}

fn view_of(json: &str) -> fdo::Result<Box<SheetView>> {
    if json.len() > MAX_VIEW {
        return Err(fdo::Error::InvalidArgs("the view is too large".to_owned()));
    }
    serde_json::from_str(json)
        .map(Box::new)
        .map_err(|e| fdo::Error::InvalidArgs(format!("not a SheetView: {e}")))
}

async fn asked(
    calls: &UnboundedSender<Call>,
    call: impl FnOnce(oneshot::Sender<fdo::Result<()>>) -> Call,
) -> fdo::Result<()> {
    let (answer, answered) = oneshot::channel();
    #[expect(
        clippy::map_err_ignore,
        reason = "a send fails only because the sheet's receiver is gone; the message says so"
    )]
    let sent = calls
        .send(call(answer))
        .map_err(|_| fdo::Error::Failed("the sheet has stopped".to_owned()));
    sent?;
    #[expect(
        clippy::map_err_ignore,
        reason = "a receive fails only because the sheet dropped the answer; the message says so"
    )]
    let received = answered
        .await
        .map_err(|_| fdo::Error::Failed("the sheet has stopped".to_owned()));
    received?
}

impl AccountsSheetServer for SheetObject {
    type Error = fdo::Error;

    async fn open(
        &self,
        BusCall { header, .. }: BusCall<'_>,
        handle: String,
        parent_window: String,
        view: String,
    ) -> fdo::Result<()> {
        // The sheet is the shell's own window: a parent window handle names nothing it needs.
        let _ = parent_window;
        let opener = from_service(&self.peers, &header).await?;
        let (handle, view) = (handle_of(&handle)?, view_of(&view)?);
        asked(&self.calls, |answer| Call::Open {
            handle,
            view,
            opener,
            answer,
        })
        .await
    }

    async fn update(
        &self,
        BusCall { header, .. }: BusCall<'_>,
        handle: String,
        view: String,
    ) -> fdo::Result<()> {
        let from = from_service(&self.peers, &header).await?;
        let (handle, view) = (handle_of(&handle)?, view_of(&view)?);
        asked(&self.calls, |answer| Call::Update {
            handle,
            view,
            from,
            answer,
        })
        .await
    }

    async fn close(&self, BusCall { header, .. }: BusCall<'_>, handle: String) -> fdo::Result<()> {
        let from = from_service(&self.peers, &header).await?;
        let handle = handle_of(&handle)?;
        asked(&self.calls, |answer| Call::Close {
            handle,
            from,
            answer,
        })
        .await
    }
}
