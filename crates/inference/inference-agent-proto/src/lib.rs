//! The two protocols an external coding agent speaks to its model endpoint, as pure functions over
//! bytes: Anthropic Messages ([`anthropic`]) and `OpenAI` Chat Completions ([`openai`]).
//!
//! Each reads a request body into the broker's own chat request ([`wire::Parsed`]) and writes a
//! reply from the broker's ([`anthropic::message_json`], [`openai::completion_json`], and a
//! [`wire::SseOut`] event stream for each). [`fail`] is how a request the endpoint cannot serve is
//! answered, in the error shape of the protocol the agent spoke.
//!
//! Nothing here binds a port, reads a socket or needs a runtime: the daemon's `agent` module owns
//! the listener, the HTTP framing, the token and the route, and calls these.

pub mod anthropic;
pub mod fail;
pub mod openai;
pub mod wire;
