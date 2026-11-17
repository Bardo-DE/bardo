//! The agent this window serves to BlueZ while the page is open: when a device pairs, BlueZ asks
//! the agent of the program that started the pairing, and the agent asks the person.

use bluez_proxy::{AgentError, Device1Proxy};
use settings_app_link::bluetooth::{PairingReply, PairingRequest};
use settings_app_model::{BtAddress, PairingAnswer, PairingAsk, PairingCode, PairingQuestion};
use tokio::sync::mpsc;
use zbus::Connection;
use zbus::zvariant::{ObjectPath, OwnedObjectPath};

/// Where the agent is served.
pub const AGENT_PATH: &str = "/org/bardo/settings/bluetooth/agent";

fn rejected() -> AgentError {
    AgentError::Rejected("The person said no.".to_owned())
}

/// The number a person typed for a passkey, or BlueZ's refusal that says why it is not one.
fn passkey_of(typed: &str) -> Result<u32, AgentError> {
    typed.parse().map_err(|why: std::num::ParseIntError| {
        AgentError::Rejected(format!("The passkey was not a number: {why}"))
    })
}

/// Forwards what devices ask to the page.
pub struct Agent {
    pub asks: mpsc::UnboundedSender<PairingRequest>,
    pub connection: Connection,
}

impl Agent {
    /// The question `ask` puts for the device at `device`.
    async fn question(&self, device: &ObjectPath<'_>, ask: PairingAsk) -> PairingQuestion {
        let proxy = match Device1Proxy::builder(&self.connection)
            .path(OwnedObjectPath::from(device.clone()))
        {
            Ok(builder) => builder.build().await.ok(),
            Err(_) => None,
        };
        let (address, name) = match proxy {
            Some(proxy) => (
                proxy.address().await.unwrap_or_default(),
                proxy.alias().await.unwrap_or_default(),
            ),
            None => (String::new(), String::new()),
        };
        let name = if name.is_empty() {
            address.clone()
        } else {
            name
        };
        PairingQuestion {
            address: BtAddress(address),
            name,
            ask,
        }
    }

    /// Puts `ask` to the person and waits for the answer; a no is BlueZ's refusal.
    async fn put(
        &self,
        device: &ObjectPath<'_>,
        ask: PairingAsk,
    ) -> Result<PairingAnswer, AgentError> {
        let question = self.question(device, ask).await;
        let (reply, answered) = PairingReply::channel();
        self.asks
            .send(PairingRequest { question, reply })
            // A closed channel means the page is gone; its send error holds only the request.
            .map_err(|_closed| rejected())?;
        match answered.await {
            Ok(PairingAnswer::No) | Err(_) => Err(rejected()),
            Ok(answer) => Ok(answer),
        }
    }

    /// Puts `ask`, which wants a yes.
    async fn confirm(&self, device: &ObjectPath<'_>, ask: PairingAsk) -> Result<(), AgentError> {
        self.put(device, ask).await.map(|_| ())
    }

    /// Puts `ask`, which wants text typed.
    async fn typed(&self, device: &ObjectPath<'_>, ask: PairingAsk) -> Result<String, AgentError> {
        match self.put(device, ask).await? {
            PairingAnswer::Typed(text) => Ok(text),
            PairingAnswer::Yes | PairingAnswer::No => Err(rejected()),
        }
    }
}

#[zbus::interface(name = "org.bluez.Agent1")]
#[expect(
    clippy::unnecessary_wraps,
    clippy::unused_self,
    reason = "zbus calls every method on the interface object, and its methods answer with `fdo::Result` so a service can refuse, whatever this one holds or does"
)]
impl Agent {
    fn release(&self) {}

    fn cancel(&self) {}

    async fn request_authorization(&self, device: ObjectPath<'_>) -> Result<(), AgentError> {
        self.confirm(&device, PairingAsk::Allow).await
    }

    async fn request_confirmation(
        &self,
        device: ObjectPath<'_>,
        passkey: u32,
    ) -> Result<(), AgentError> {
        self.confirm(&device, PairingAsk::Compare(PairingCode(passkey)))
            .await
    }

    /// A number to type on the device. BlueZ repeats the call as digits are typed; only the first
    /// opens the question.
    async fn display_passkey(&self, device: ObjectPath<'_>, passkey: u32, entered: u16) {
        if entered == 0 {
            let question = self
                .question(&device, PairingAsk::Show(PairingCode(passkey)))
                .await;
            let (reply, _heard) = PairingReply::channel();
            let _ = self.asks.send(PairingRequest { question, reply });
        }
    }

    fn authorize_service(&self, device: ObjectPath<'_>, uuid: &str) -> Result<(), AgentError> {
        // Every service of a paired device is allowed: the pairing was what the person approved.
        let _ = (device, uuid);
        Ok(())
    }

    /// A PIN of up to sixteen characters, typed by the person.
    async fn request_pin_code(&self, device: ObjectPath<'_>) -> Result<String, AgentError> {
        self.typed(&device, PairingAsk::PinCode).await
    }

    /// A number of up to six digits, typed by the person.
    async fn request_passkey(&self, device: ObjectPath<'_>) -> Result<u32, AgentError> {
        passkey_of(&self.typed(&device, PairingAsk::Passkey).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_passkey_that_is_not_a_number_says_why() {
        assert!(matches!(passkey_of("123456"), Ok(123_456)));
        let Err(AgentError::Rejected(words)) = passkey_of("12x") else {
            panic!("a passkey with a letter in it must be refused");
        };
        assert!(words.contains("invalid digit"), "{words}");
    }
}
