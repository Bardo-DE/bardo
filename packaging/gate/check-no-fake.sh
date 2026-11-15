#!/usr/bin/env bash
# No stand-in ships. Several crates (turnstile, peephole, keycap, palmrest) carry a `fake` feature: a fake kernel, a
# made-up caller identity. It exists for tests, and a program built with it would let anything pass as anyone, so no
# command that builds what the desktop ships may switch it on. Two places are read:
#
#   1. the scripts and workflows that build or install the desktop (install.sh, install/, release/, harness/, the
#      workflows; gate/ is left out): no line that runs cargo (or this tree's cargo helpers) names `fake` or asks for
#      `--all-features`;
#   2. every Cargo.toml of the tree (--tree): `fake` is not in a crate's default features (followed through the
#      features that name it) and no normal or build dependency, or workspace dependency, turns it on. A
#      dev-dependency may: tests need it, and cargo does not unify it into a build of the package itself.
#
#   gate/check-no-fake.sh [--tree DIR] [--no-crates]
#
#   --tree DIR      the bardo git tree (default: the parent of this folder)
#   --no-crates     read only 1
# Exit: 0 clean, 1 something found (each printed), 2 usage.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
usage() { sed -n '2,/^set -uo/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }
tree="$(dirname "$ROOT")" crates=1
while [ $# -gt 0 ]; do
  case "$1" in
    --tree) tree=${2:?}; shift ;;
    --no-crates) crates=0 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
  shift
done
command -v python3 >/dev/null || { echo "check-no-fake: python3 is not installed" >&2; exit 2; }
exec python3 -I - "$ROOT" "$tree" "$crates" <<'PY'
import os
import re
import subprocess
import sys
import tomllib

root, tree, crates = sys.argv[1], sys.argv[2], sys.argv[3] == "1"
found = []


def bad(message):
    found.append(message)


# 1. The build and install scripts and workflows.
RUNS_CARGO = re.compile(r"\bcargo(_in)?\b|\bbuild_tree\b|\bbuild_parts\b|\bcargo-[a-z]+\b")
# The stand-in features: peephole/turnstile/keycap/palmrest `fake`, and peephole's `dev-rows` (a person's own rows
# file naming services).
STAND_INS = {"fake", "dev-rows"}
NAMES_FAKE = re.compile(r"\bfake\b|\bdev-rows\b|--all-features")
scripts = [os.path.join(root, "install.sh")]
for folder in ("install", "release", ".github/workflows", "harness"):
    for dirpath, _, files in os.walk(os.path.join(root, folder)):
        for name in sorted(files):
            if name.endswith((".sh", ".yml", ".yaml", ".py")):
                scripts.append(os.path.join(dirpath, name))
for path in scripts:
    if not os.path.isfile(path):
        continue
    # The release's own self-test packs a made-up release: its word "fake" is not a cargo feature.
    if os.path.basename(path) in ("selftest.sh", "selftest-forge.py", "health-scenarios.sh"):
        continue
    with open(path, encoding="utf-8", errors="replace") as f:
        for number, line in enumerate(f, 1):
            code = line.split("#", 1)[0]
            if RUNS_CARGO.search(code) and NAMES_FAKE.search(code):
                bad(f"{os.path.relpath(path, root)}:{number}: a build command names the fake feature or all features: {line.strip()}")

# 2. The crates.
def reaches_fake(features, start="default", seen=None):
    seen = set() if seen is None else seen
    if start in seen:
        return False
    seen.add(start)
    for item in features.get(start, []):
        name = item.split("/")[-1].removeprefix("dep:")
        if name in STAND_INS and ("/" in item or item in STAND_INS):
            return True
        if item in features and reaches_fake(features, item, seen):
            return True
    return False


def turns_on(spec):
    return isinstance(spec, dict) and any(f in STAND_INS for f in spec.get("features", []))


def check_manifest(path, label):
    with open(path, "rb") as f:
        doc = tomllib.load(f)
    features = doc.get("features", {})
    if reaches_fake(features):
        bad(f"{label}: `fake` is in the default features")
    tables = [("dependencies", doc.get("dependencies", {})), ("build-dependencies", doc.get("build-dependencies", {})),
              ("workspace.dependencies", doc.get("workspace", {}).get("dependencies", {}))]
    for target in doc.get("target", {}).values():
        tables.append(("target dependencies", target.get("dependencies", {})))
    for kind, table in tables:
        for name, spec in table.items():
            if turns_on(spec):
                bad(f"{label}: the {kind} entry {name} turns `fake` on")


if crates and os.path.isdir(tree):
    listing = subprocess.run(["git", "-C", tree, "ls-files", "--cached", "--others", "--exclude-standard", "--", "Cargo.toml", "**/Cargo.toml"],
                             capture_output=True, text=True)
    if listing.returncode != 0:
        bad(f"cannot list the files of {tree} ({listing.stderr.strip()}); its crates were not read")
    else:
        for rel in sorted(set(listing.stdout.split())):
            try:
                check_manifest(os.path.join(tree, rel), rel)
            except (OSError, tomllib.TOMLDecodeError) as e:
                bad(f"{rel}: cannot be read: {e}")

if found:
    for line in found:
        print(f"check-no-fake: {line}")
    print(f"check-no-fake: FAIL ({len(found)})")
    sys.exit(1)
print("check-no-fake: ok (no shipped build command or crate turns the fake feature on)")
PY
