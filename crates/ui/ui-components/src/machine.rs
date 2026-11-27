//! A [`Machine`](base_machine::machine::Machine) as a hook: [`use_machine`] keeps the state, seeded by
//! the caller, and [`use_machine_in`] runs a machine over a [`MachineState`] the caller owns.
//!
//! The hooks live in `ui_motion::machine`, below the hooks that run on them (presence, rosters, the
//! swipe); this is their name in `ds` for surfaces and apps. The trait, `Elapsed` and `Stamp` are
//! `base-machine`'s (`base_machine::machine`, `base_machine::time::stamp`).

pub use ui_motion::machine::{
    MachineRef, MachineState, use_machine, use_machine_in, use_machine_state,
};
