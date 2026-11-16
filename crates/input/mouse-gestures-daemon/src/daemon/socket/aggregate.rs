//! What the subscribers jointly say: one answer from many.

/// The pointer is ours if any subscriber says so.
pub(super) fn aggregate(overs: impl IntoIterator<Item = bool>) -> bool {
    overs.into_iter().any(|over| over)
}

/// Whether the session counts as locked, from what each subscriber last said (`None`: nothing
/// yet). Locked is the safe answer, so it wins whenever it is in doubt: any subscriber says
/// locked, or nobody has said unlocked (no subscriber is connected, or the ones connected have
/// said nothing). A lock screen the shell has not reported must never let a gesture run a key or
/// a desktop command. Only a subscriber saying unlocked, with none saying locked, opens it.
pub(super) fn aggregate_lock(locks: impl IntoIterator<Item = Option<bool>>) -> bool {
    let said: Vec<bool> = locks.into_iter().flatten().collect();
    !said.contains(&false) || said.contains(&true)
}
