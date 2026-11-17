//! `org.bardo.IntentProvider1` served for one provider: each member decodes the JSON of its
//! typed argument, calls the provider and answers the JSON of its typed result. The member
//! list, its signatures and its signals are those of `action-dbus`'s `IntentProviderSkeleton`
//! (a test holds the introspection to `dbus/org.bardo.IntentProvider1.xml`).
//!
//! The gate lives in intentd: a provider answers nothing but intentd, so a process that calls
//! an app's bus name directly gets no way around it. Intentd is the process porter's callers table
//! names as the router (its unit, `bardo-actiond.service`, by the main process the systemd manager
//! reports), never whoever owns a well-known name. `Summon` is the one member that also comes
//! from the shell.

use crate::error::ServeError;
use crate::provider::{ContextSource, IntentProvider, SummonTarget};
use action_dbus::callers::{Callers, INTENTD, Identity, intentd_only, require};
use action_dbus::{BusConnection, Details, PROVIDER_PATH};
use action_manifest::ValidManifest;
use action_wire::SummonSerial;
use futures_util::StreamExt;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use zbus::fdo;
use zbus::message::Header;
use zbus::object_server::SignalEmitter;

fn parse<T: DeserializeOwned>(text: &str) -> fdo::Result<T> {
    serde_json::from_str(text).map_err(|e| fdo::Error::InvalidArgs(e.to_string()))
}

fn render<T: Serialize>(value: &T) -> fdo::Result<String> {
    serde_json::to_string(value).map_err(|e| fdo::Error::Failed(e.to_string()))
}

/// One provider with the two seams ds answers for it.
struct ProviderObject<P, C, S, K> {
    provider: P,
    context: C,
    summon: S,
    callers: Arc<K>,
}

/// Whether the caller is intentd: the process `callers` names as the router.
async fn from_intentd(callers: &impl Callers, header: &Header<'_>) -> fdo::Result<()> {
    require(
        callers,
        header.sender().map(zbus::names::UniqueName::as_str),
        &[INTENTD],
    )
    .await
    .map(drop)
    .map_err(|refused| {
        fdo::Error::AccessDenied(format!("only intentd calls a provider ({refused})"))
    })
}

#[zbus::interface(name = "org.bardo.IntentProvider1")]
#[expect(
    clippy::unused_self,
    clippy::needless_pass_by_value,
    reason = "the zbus interface macro calls every method and property getter on the interface object and hands a method its arguments by value, whatever this one holds or does"
)]
impl<P, C, S, K> ProviderObject<P, C, S, K>
where
    P: IntentProvider + 'static,
    C: ContextSource + 'static,
    S: SummonTarget + 'static,
    K: Callers,
{
    async fn perform(
        &self,
        invocation: String,
        options: Details,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        let _ = options;
        from_intentd(self.callers.as_ref(), &header).await?;
        let delivered: action_core::ActivatedInvocation = parse(&invocation)?;
        render(
            &self
                .provider
                .perform_classified(
                    delivered.invocation,
                    delivered.activation,
                    delivered.classified,
                )
                .await,
        )
    }

    async fn classify(
        &self,
        invocation: String,
        options: Details,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        let _ = options;
        from_intentd(self.callers.as_ref(), &header).await?;
        render(&self.provider.classify(parse(&invocation)?).await)
    }

    async fn dry_run(
        &self,
        invocation: String,
        options: Details,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        let _ = options;
        from_intentd(self.callers.as_ref(), &header).await?;
        render(&self.provider.dry_run(parse(&invocation)?).await)
    }

    async fn undo(
        &self,
        token: String,
        actor: String,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        from_intentd(self.callers.as_ref(), &header).await?;
        render(&self.provider.undo(parse(&token)?, parse(&actor)?).await)
    }

    async fn context(&self, #[zbus(header)] header: Header<'_>) -> fdo::Result<String> {
        from_intentd(self.callers.as_ref(), &header).await?;
        render(&self.context.snapshot())
    }

    fn summon(&self, serial: u64, origin: String) -> fdo::Result<String> {
        render(&self.summon.summon(SummonSerial(serial), parse(&origin)?))
    }

    async fn search(
        &self,
        text: String,
        generation: u64,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        let _ = generation;
        from_intentd(self.callers.as_ref(), &header).await?;
        render(&self.provider.search(&text).await)
    }

    async fn preview(
        &self,
        entity: String,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        from_intentd(self.callers.as_ref(), &header).await?;
        render(&self.provider.preview(&parse(&entity)?).await)
    }

    async fn suggest(
        &self,
        ask: String,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        from_intentd(self.callers.as_ref(), &header).await?;
        render(&self.provider.suggest(parse(&ask)?).await)
    }

    // The ids-then-metas split has no provider method yet: nothing resolves keys. (A plain
    // comment: a doc comment would land in the introspection.)
    fn resolve(&self, keys: String) -> fdo::Result<String> {
        let _ = keys;
        Err(fdo::Error::NotSupported(
            "this provider resolves no keys".into(),
        ))
    }

    #[zbus(signal)]
    async fn undo_changed(
        emitter: &SignalEmitter<'_>,
        token: &str,
        state: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn index_stale(emitter: &SignalEmitter<'_>, epoch: u64) -> zbus::Result<()>;
}

/// Exports `provider` at `/org/bardo/IntentProvider1` on `connection` and claims the manifest's app
/// as its bus name.
///
/// Only intentd's unit (`bardo-actiond.service`, by its main process, over the system's `/proc`) is
/// answered. Returns once the name is ours; the connection keeps serving.
///
/// # Errors
///
/// Returns `ServeError::Bus` when the bus refuses or fails, and `ServeError::Taken` when another
/// process already owns the app's bus name.
pub async fn serve_on<P, C, S>(
    connection: &BusConnection,
    provider: P,
    context: C,
    summon: S,
) -> Result<(), ServeError>
where
    P: IntentProvider + 'static,
    C: ContextSource + 'static,
    S: SummonTarget + 'static,
{
    let callers = Arc::new(Identity::on_system(connection.clone(), intentd_only()));
    serve_on_with(connection, provider, context, summon, callers).await
}

/// [`serve_on`], answering the process that `callers` names as intentd (a test's fake proc root,
/// or a host with a table of its own).
///
/// # Errors
///
/// Returns the same errors as `serve_on`.
pub async fn serve_on_with<P, C, S, K>(
    connection: &BusConnection,
    provider: P,
    context: C,
    summon: S,
    callers: Arc<K>,
) -> Result<(), ServeError>
where
    P: IntentProvider + 'static,
    C: ContextSource + 'static,
    S: SummonTarget + 'static,
    K: Callers,
{
    let manifest: &ValidManifest = provider.manifest();
    let name = manifest.manifest().app.to_string();
    let object = ProviderObject {
        provider,
        context,
        summon,
        callers,
    };
    connection.object_server().at(PROVIDER_PATH, object).await?;
    dbus_kit::ready::serving(connection).await?;
    match dbus_kit::name::claim(connection, name.as_str()).await {
        Ok(()) => Ok(()),
        Err(dbus_kit::name::ClaimError::Taken(_)) => Err(ServeError::Taken(name)),
        Err(dbus_kit::name::ClaimError::Bus { source, .. }) => Err(source.into()),
    }
}

/// Returns when the connection closes.
pub(crate) async fn until_closed(connection: &BusConnection) {
    let mut messages = zbus::MessageStream::from(connection);
    while messages.next().await.is_some() {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use action_core::{
        AppRefusal, ContextSnapshot, Hit, Invocation, Outcome, Preview, SuggestAsk, UndoFault,
    };
    use action_vocab::{EntityRef, UndoToken};
    use action_wire::{SummonAnswer, SummonOrigin};
    use provenance::{Actor, EntityId};
    use zbus::object_server::Interface;

    struct Nobody(ValidManifest);

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async and this fake never waits"
    )]
    impl IntentProvider for Nobody {
        fn manifest(&self) -> &ValidManifest {
            &self.0
        }
        async fn perform(&self, _: Invocation) -> Result<Outcome, AppRefusal> {
            Err(AppRefusal::Unsupported)
        }
        async fn dry_run(&self, _: Invocation) -> Result<Preview, AppRefusal> {
            Err(AppRefusal::Unsupported)
        }
        async fn undo(&self, _: UndoToken, _: Actor) -> Result<(), UndoFault> {
            Err(UndoFault::Gone)
        }
        async fn search(&self, _: &str) -> Vec<Hit> {
            vec![]
        }
        async fn preview(&self, _: &EntityId) -> Preview {
            Preview::None
        }
        async fn suggest(&self, _: SuggestAsk) -> Vec<EntityRef> {
            vec![]
        }
    }

    /// Identifies nobody: the introspection needs the type, not a bus.
    struct NoCallers;
    impl Callers for NoCallers {
        #[expect(
            clippy::unused_async_trait_impl,
            reason = "the trait method is async and this fake never waits"
        )]
        async fn caller_of(&self, _: &str) -> Option<action_dbus::callers::Caller> {
            None
        }
    }

    struct Blank;
    impl ContextSource for Blank {
        fn snapshot(&self) -> ContextSnapshot {
            unreachable!("introspection only")
        }
    }
    impl SummonTarget for Blank {
        fn summon(&self, _: SummonSerial, _: SummonOrigin) -> SummonAnswer {
            unreachable!("introspection only")
        }
    }

    #[test]
    fn the_served_interface_is_the_declared_one() {
        let manifest = action_manifest::validate(action_manifest::Manifest {
            vocab: action_vocab::IntentsVocab(1),
            app: provenance::AppName::parse("org.bardo.Nobody").expect("an app name"),
            entities: vec![],
            actions: vec![],
            visibility: action_manifest::Visibility::Everyone,
        })
        .expect("a manifest");
        let object = ProviderObject {
            provider: Nobody(manifest),
            context: Blank,
            summon: Blank,
            callers: Arc::new(NoCallers),
        };
        let mut served = String::from(
            "<!DOCTYPE node PUBLIC \"-//freedesktop//DTD D-BUS Object Introspection 1.0//EN\"\n \"http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd\">\n<node>\n",
        );
        object.introspect_to_writer(&mut served, 1);
        served.push_str("</node>\n");
        let declared = include_str!("../../dbus/org.bardo.IntentProvider1.xml");
        assert_eq!(served, declared);
    }
}
