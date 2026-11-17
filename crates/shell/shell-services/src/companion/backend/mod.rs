//! The service's effects: the session bus, as the shell.

mod answer;
mod computer_use;
mod helpers;
mod intents;
mod memory;
mod pool;
mod probe;
mod proposals;
mod routines;
mod run;
mod shown;
mod tell;
mod terminal;
mod terminals;
mod text;

pub(crate) use run::{Reach, run};
