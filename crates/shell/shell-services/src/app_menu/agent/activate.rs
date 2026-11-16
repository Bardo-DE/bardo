//! Activating an item for an agent.

use std::collections::BTreeMap;

use action_client::Transport;
use action_vocab::{ActionRef, CallClass, CallRequest, Origin, TargetValue};
use provenance::Effect;
use shell_model::app_menu::{
    ActionIndex, Identity, ItemKey, Route, effect_of, own_commands, route_of,
};
use shell_model::tray::{MenuCommand, TrayMenuEntry, TrayMenuKind};

use super::{MenuAgent, Target};
use crate::app_menu::wire;

/// Why an ask about a menu did not happen. The words never carry the app's own text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuFault {
    /// No such window.
    NoWindow,
    /// The window's app serves no menu.
    NoMenu,
    /// No item has that key.
    NoSuchItem,
    /// The item is disabled, or opens a submenu and runs nothing.
    NotRunnable,
    /// The item is no longer what the router gated the call on.
    Changed,
    /// Something else failed: `words` is what the person may be told, `cause` what went wrong
    /// (for the log; it may carry the app's own text, so it is never shown).
    Failed {
        /// What the person may be told.
        words: String,
        /// What went wrong, for the log.
        cause: String,
    },
}

impl MenuFault {
    /// A failure told as `words`, caused by `cause`.
    pub(super) fn failed(words: &str, cause: impl std::fmt::Display) -> Self {
        Self::Failed {
            words: words.to_owned(),
            cause: cause.to_string(),
        }
    }
}

/// What an activation did and what the table said it costs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activation {
    pub how: Activated,
    /// The effect of the route taken (`shell_model::app_menu::effect_of`).
    pub effect: Effect,
}

/// How an activation was carried out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Activated {
    /// The shell did one of its own rows.
    Own,
    /// The verified app's action, to be made as the next call (the router gates it on its own).
    Delegated(CallRequest),
    /// The app was sent a click it must weigh itself.
    Clicked,
}

/// What an item is now, worked out from the live menu and the proof of the app: how it would be
/// carried out, and what it costs.
#[derive(Debug)]
struct Resolved {
    route: Route,
    effect: Effect,
    /// The bus name's owner, for a click.
    owner: Option<String>,
    /// The windows as they are, for the shell's own rows.
    target: Target,
}

impl Resolved {
    /// What the router is told about the item.
    fn class(&self) -> CallClass {
        match &self.route {
            Route::Intents(action) => CallClass::Delegates(action.clone()),
            Route::Own(_) | Route::Click(_) => CallClass::Effect(self.effect),
        }
    }
}

impl<T: Transport> MenuAgent<T> {
    /// What `item` of the window asked for would be, as the router gates it
    /// (`shell.menu.activate` is `per_call = "classified"`): its effect for a row the shell does
    /// itself, a delegation to the verified app's own action, otherwise `Destructive`.
    pub async fn classify(&self, window: Option<&str>, item: &str) -> Result<CallClass, MenuFault> {
        self.resolve(window, item).await.map(|r| r.class())
    }

    /// Activate `item` of the window asked for (the focused one when `window` is `None`).
    /// `classified` is what the router gated the call on, when it says: the item is worked out
    /// again, and a call that is no longer that is `MenuFault::Changed`, except `Destructive`,
    /// the declared effect, which is always accepted. A verified app's own action is not done
    /// here: it is handed back as the call to make next, which the router gates on its own.
    pub async fn activate(
        &self,
        window: Option<&str>,
        item: &str,
        classified: Option<&CallClass>,
    ) -> Result<Activation, MenuFault> {
        let now = self.resolve(window, item).await?;
        let ceiling = CallClass::Effect(Effect::Destructive);
        if let Some(said) = classified
            && *said != ceiling
            && *said != now.class()
        {
            return Err(MenuFault::Changed);
        }
        let effect = now.effect;
        let how = match now.route {
            Route::Own(own) => {
                for command in own_commands(own, &now.target.app.0, &now.target.toplevels) {
                    self.links.wl.send(command).map_err(|_closed| {
                        MenuFault::failed(
                            "the compositor is not there",
                            "the compositor's command channel is closed",
                        )
                    })?;
                }
                Activated::Own
            }
            Route::Intents(action) => Activated::Delegated(CallRequest {
                action,
                target: TargetValue::Nothing,
                args: BTreeMap::new(),
                origin: Origin::AppInternal,
            }),
            Route::Click(id) => {
                let owner = now.owner.ok_or(MenuFault::NoMenu)?;
                wire::click(&self.connection, &owner, id)
                    .await
                    .map_err(|why| MenuFault::failed("the app could not be told", why))?;
                Activated::Clicked
            }
        };
        Ok(Activation { how, effect })
    }

    async fn resolve(&self, window: Option<&str>, item: &str) -> Result<Resolved, MenuFault> {
        let key = ItemKey::parse(item).ok_or(MenuFault::NoSuchItem)?;
        let target = self.target(window)?;
        let id = match key {
            ItemKey::Own(own) => {
                let route = Route::Own(own);
                let effect = effect_of(&route, &ActionIndex::default());
                return Ok(Resolved {
                    route,
                    effect,
                    owner: None,
                    target,
                });
            }
            ItemKey::Menu(id) => id,
        };
        let served = self.serve(&target.app).await?.ok_or(MenuFault::NoMenu)?;
        let row = find(&served.menu.menus, id).ok_or(MenuFault::NoSuchItem)?;
        let command = runnable(row)?;
        let identity = self.identity(&served).await;
        let actions = self.actions(command, &identity).await?;
        let route = route_of(key, command, &identity, &actions);
        let effect = effect_of(&route, &actions);
        Ok(Resolved {
            route,
            effect,
            owner: Some(served.owner.as_str().to_owned()),
            target,
        })
    }

    /// The installed actions and their effects; none are read unless a proven app names one.
    async fn actions(
        &self,
        command: Option<&MenuCommand>,
        identity: &Identity,
    ) -> Result<ActionIndex, MenuFault> {
        if command.is_none() || !matches!(identity, Identity::Verified(_)) {
            return Ok(ActionIndex::default());
        }
        let manifests = self
            .intents
            .manifests()
            .await
            .map_err(|why| MenuFault::failed("intentd could not be asked", why))?;
        let actions: BTreeMap<ActionRef, _> = manifests
            .iter()
            .flat_map(|valid| {
                let manifest = valid.manifest();
                manifest.actions.iter().map(|action| {
                    (
                        ActionRef {
                            app: manifest.app.clone(),
                            name: action.name.clone(),
                        },
                        action.effect,
                    )
                })
            })
            .collect();
        Ok(ActionIndex(actions))
    }
}

/// The row with dbusmenu id `id`, anywhere in `rows`.
fn find(rows: &[TrayMenuEntry], id: i32) -> Option<&TrayMenuEntry> {
    rows.iter().find_map(|row| {
        if row.id == id {
            return Some(row);
        }
        match &row.kind {
            TrayMenuKind::Item { children, .. } => find(children, id),
            TrayMenuKind::Separator => None,
        }
    })
}

/// `row`'s command claim, when it is a row that runs something: enabled and no submenu.
fn runnable(row: &TrayMenuEntry) -> Result<Option<&MenuCommand>, MenuFault> {
    match &row.kind {
        TrayMenuKind::Item {
            enabled: true,
            children,
            command,
            ..
        } if children.is_empty() => Ok(command.as_ref()),
        TrayMenuKind::Item { .. } | TrayMenuKind::Separator => Err(MenuFault::NotRunnable),
    }
}

#[cfg(test)]
mod tests {
    use super::MenuFault;

    #[test]
    fn a_failure_keeps_its_cause_apart_from_the_words() {
        let fault = MenuFault::failed("the app could not be told", "the bus: no reply");
        assert_eq!(
            fault,
            MenuFault::Failed {
                words: "the app could not be told".to_owned(),
                cause: "the bus: no reply".to_owned(),
            }
        );
    }
}
