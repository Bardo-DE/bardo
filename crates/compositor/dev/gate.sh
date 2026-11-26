#!/usr/bin/env bash
# The compositor's gate (20-FREEZE-PLAN §6.2): `dev/gate.sh fast -p <crate>… | merge | phase`.
# Every exit code checked; never piped, never tailed. fmt, clippy, deny and the layer rules are
# `cargo xtask gate`'s; this gate is the jail, the tests inside it, acceptance and wlcs.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${CARGO_TARGET_DIR:=$PWD/../../target}"; export CARGO_TARGET_DIR
mkdir -p "$CARGO_TARGET_DIR"
tier="${1:?fast|merge|phase}"; shift
export GATE_TIER="$tier"
run=$(dev/lib/run-id.sh)
mkdir -p "dev/out/$run"
step() { dev/lib/step.sh "$run" "$@"; }       # times, writes summary.json, applies budgets.toml
# §6.2 wrote `jail` as a shell function, which step.sh (its own process) cannot call; the same
# command line as an array.
jail=("${BARDO_COMPOSITOR_JAIL:-../../packaging/harness/jail.sh}" --fabricate-gpus 2 --bind "$PWD/../.." --bind "$CARGO_TARGET_DIR" --)
step preflight  dev/preflight.sh
case "$tier" in
  fast)
    step archive  cargo nextest archive "$@" --all-features --archive-file "dev/out/$run/tests.tar.zst"
    step test     "${jail[@]}" cargo-nextest nextest run --profile gate --archive-file "dev/out/$run/tests.tar.zst" --workspace-remap "$PWD" ;;
  merge|phase)
    step doc      cargo test --doc --workspace --all-features
    step archive  cargo nextest archive --workspace --all-features --archive-file "dev/out/$run/tests.tar.zst"
    step test     "${jail[@]}" cargo-nextest nextest run --profile gate --archive-file "dev/out/$run/tests.tar.zst" --workspace-remap "$PWD"
    step accept   "${jail[@]}" dev/accept.sh --jobs 8 ;;
  *) echo "gate: unknown tier '$tier' (fast|merge|phase)" >&2; exit 64 ;;
esac
if [ "$tier" = phase ]; then
  step wlcs        "${jail[@]}" dev/wlcs/run.sh --phase
fi
echo "GATE GREEN ($tier) $run"
