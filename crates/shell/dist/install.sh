#!/usr/bin/env bash
# Install bardo-shell from the bardo tree: build `bardo-shell` in release, then put the binary, the PAM service,
# the intents manifest (`org.bardo.Shell`, for actiond and `bardo-do`), the companion skills
# (dist/skills) and the settings schema where a login reads them, and copy the key remap daemon layer for
# the bardo-shell session to ~/.config/bardo/key-remap/key-remap.toml when that file is missing (never over yours).
# The login itself, "Bardo", is the desktop repo's session/ and install/. Run as
# yourself; the root step asks once. Safe to run again.
#
#   dist/install.sh --dry-run    print every step, change nothing (no build; the lock check still runs)
#   dist/install.sh              do it
#   --prefix DIR                 install under DIR (default /usr/local): DIR/bin, DIR/share/bardo
#   --destdir DIR                stage under DIR instead of the real system (packaging): nothing is
#                                escalated, and the key remap daemon layer, udev reload and user manager are skipped
#   --no-build                   do not run cargo; use the release binary already built
#
# Root: $SUDO if set, else sudo, doas or pkexec; with none of them the commands that need root are
# printed at the end for you to run. The build links with this system's own linker: it runs from /
# with --manifest-path (install-lib.sh cargo_outside), so the repo's .cargo/config.toml, which asks for
# clang and mold for development, is not read. Nothing is installed for the linker.
#
# Also installed, when no ddcutil rule exists on the machine (60-ddcutil*.rules): dist/udev and
# dist/modules-load.d, so the i2c buses of external monitors open to the person at the seat (DDC/CI
# brightness). Not installed here, on purpose: dist/bardo-shell.service (the desktop repo's installer puts it root-held beside
# bardo-compositor.service and its session starts it; a copy under ~/.config is not trusted by the account service),
# anything of Plasma's or Toshy's, and the key remap daemon's machine file /etc/bardo/key-remap/key-remap.toml.
# dist/uninstall.sh undoes it. docs/first-login.md is next.
set -euo pipefail
# shellcheck source=install-lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/install-lib.sh"

usage() { awk 'NR > 1 { if (!/^#/) exit; sub(/^# ?/, ""); print }' "${BASH_SOURCE[0]}"; }
HAS_BUILD=1
parse_flags "$@"
[ "$DRY" = 0 ] || echo "dry run: nothing below is executed"

step "1/8 checks"
refuse_root
[ "$NO_BUILD" = 1 ] || command -v cargo >/dev/null || { echo "cargo not found (install rustup: the repo pins its Rust version)" >&2; exit 1; }
if [ -e "$HOME/.cargo/bin/bardo-shell" ] && [ -z "$DESTDIR" ]; then
  note "WARNING: $HOME/.cargo/bin/bardo-shell would come first on a login shell's PATH and shadow $BIN_DEST;"
  note "         remove it (rm $HOME/.cargo/bin/bardo-shell) or the session runs the old build"
fi
need_root_helper

step "2/8 the lock file against the workspace"
if [ "$NO_BUILD" = 1 ]; then note "skipped: --no-build"; else check_lock; fi

step "3/8 build bardo-shell (release, from $TREE)"
if [ "$NO_BUILD" = 1 ]; then
  note "skipped: --no-build"
else
  run cargo_outside build --release --locked --manifest-path "$TREE/Cargo.toml" -p shell-app
fi
BUILT="${CARGO_TARGET_DIR:-$TREE/target}/release/bardo-shell"
[ "$DRY" = 1 ] || [ -x "$BUILT" ] || { echo "no binary at $BUILT" >&2; exit 1; }

step "4/8 the binary"
put 755 "$BUILT" "$BIN_DEST"

step "5/8 the PAM service, the intents manifest, the skills and the settings schema"
put 644 "$DIST/pam.d/bardo-shell" "$PAM_DEST"
# actiond (the actions service) lists it, and `bardo-do shell --list` shows it: bardo-shell serves it as `org.bardo.Shell`.
put 644 "$DIST/intents/org.bardo.Shell.toml" "$INTENTS_DEST"
# The skills the companion loads (agent-eval --check-skills validates them); they grant nothing.
while IFS= read -r file; do
  put 644 "$DIST/skills/$file" "$SKILLS_DEST/$file"
done < <(skills)
# The settings schema, written by the binary just built, so it always matches it.
schema_dir=$(mktemp -d)
# With no root helper the printed commands still need the file, so it stays then.
trap '[ "$ESC_MODE" = print ] || rm -rf "$schema_dir"' EXIT
if [ "$DRY" = 1 ]; then
  note "would write the settings schema with $BUILT --write-schema and install it at $SCHEMA_DEST"
else
  "$BUILT" --write-schema "$schema_dir" >/dev/null
  put 644 "$schema_dir/shell.settings.toml" "$SCHEMA_DEST"
fi

step "6/8 external monitor brightness (DDC/CI): the i2c rule and module"
if ddcutil_rule_present "$DESTDIR"; then
  note "a ddcutil udev rule exists: bardo-shell adds none (the ddcutil package loads i2c-dev as well)"
else
  put 644 "$DDC_RULE_SRC" "$DDC_RULE_DEST"
  put 644 "$DDC_MODULES_SRC" "$DDC_MODULES_DEST"
  if [ -z "$DESTDIR" ] && [ "${#INSTALLED[@]}" -gt 0 ] && [[ " ${INSTALLED[*]} " == *" $DDC_RULE_DEST "* ]]; then
    as_root udevadm control --reload-rules
    as_root udevadm trigger --subsystem-match=i2c-dev --action=add
    as_root modprobe i2c-dev || note "i2c-dev did not load now; it loads at the next boot"
  fi
fi

step "7/8 the key_remap layer for the bardo-shell session (copied when missing; never overwritten)"
if [ -n "$DESTDIR" ]; then
  note "skipped: a staging root has no person's home"
elif [ -f "$BARDO_KEY_REMAP_LAYER_DEST" ] && cmp -s "$BARDO_KEY_REMAP_LAYER_SRC" "$BARDO_KEY_REMAP_LAYER_DEST"; then
  note "unchanged: $BARDO_KEY_REMAP_LAYER_DEST"
elif [ -e "$BARDO_KEY_REMAP_LAYER_DEST" ]; then
  note "kept (yours): $BARDO_KEY_REMAP_LAYER_DEST"
  note "YOUR KEYBOARD FILE IS OLDER THAN THIS ONE ($BARDO_KEY_REMAP_LAYER_SRC). Install never changes your file."
  lacking="$(key_remap_rules_lacking)"
  if [ -n "$lacking" ]; then
    note "These rules are in the new file and not in yours (a missing one can send a shortcut to the wrong place, such as Cmd+Tab; the app switcher rules keep Super down while Cmd is held):"
    while IFS= read -r rule; do note "  $rule"; done <<<"$lacking"
  else
    note "Your file has every rule of the new one; it differs only in comments or extra rules of yours."
  fi
  note "To see every difference:  diff $BARDO_KEY_REMAP_LAYER_DEST $BARDO_KEY_REMAP_LAYER_SRC"
  note "To use the new file (keeps yours as key-remap.toml.bak):  cp $BARDO_KEY_REMAP_LAYER_DEST $BARDO_KEY_REMAP_LAYER_DEST.bak && cp $BARDO_KEY_REMAP_LAYER_SRC $BARDO_KEY_REMAP_LAYER_DEST"
else
  run install -D -m 644 "$BARDO_KEY_REMAP_LAYER_SRC" "$BARDO_KEY_REMAP_LAYER_DEST"
fi

step "8/8 SELinux labels and the user manager"
if [ -n "$DESTDIR" ]; then
  note "skipped: a staging root"
else
  if ! command -v restorecon >/dev/null; then
    note "no restorecon: skipped"
  elif [ "${#INSTALLED[@]}" -gt 0 ]; then
    as_root restorecon -F "${INSTALLED[@]}"
  fi
  run systemctl --user daemon-reload
fi

step "done"
print_pending
note "Next: install the \"Bardo\" login from the desktop repo, then docs/first-login.md. Plasma is untouched and stays in the session list."
