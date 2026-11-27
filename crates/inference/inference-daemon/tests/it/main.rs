//! The crate's integration tests: one executable, one module per topic.
//! `accept::attached`, `engine_start` and `accept::shutdown` start this test executable again as
//! a fake child, by test path (`accept::shutdown::fake_inferd`).

mod accept;
mod agents;
mod agents_local;
mod broker;
mod cloud;
mod daemon;
mod dist;
mod engine_start;
mod hosted;
mod hosting;
mod isolation;
mod local_engines;
mod own_server;
mod places;
mod probed;
mod replayed;
mod scores;
mod serve;
mod settings_module;
mod speech;
mod start;
mod support;
mod tailnet;
mod voice_chat;
