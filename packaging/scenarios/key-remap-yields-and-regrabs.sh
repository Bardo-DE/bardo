#!/usr/bin/env bash
# Scenario `bardo-key-remap-yields-and-regrabs` (plan §4.14): bardo-key-remap Yield and re-grab around a bardo-compositor crash, with a fake bardo-key-remap socket; needs bardo-key-remap v2
# Not runnable yet: exits 2 (SKIP), listed in gate/expected-skips.toml, closed by P4.
# Contract: require_jail first, only harness/lib, exits with plan §6.1's codes.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../harness/lib/all.sh"
require_jail
echo "SKIP bardo-key-remap-yields-and-regrabs: not runnable until P4 (bardo-key-remap Yield and re-grab around a bardo-compositor crash, with a fake bardo-key-remap socket; needs bardo-key-remap v2)" >&2
exit 2
