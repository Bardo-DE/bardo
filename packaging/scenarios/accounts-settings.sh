#!/usr/bin/env bash
# Scenario `accounts-settings` (design 31 §7.1, PLAN §2.5): bardo-settings's Accounts pane against the live daemons. Each control
# is clicked in the real window on bardo-compositor (the window is placed by the compositor; clicks are relative to its frame,
# `bardo-compositorctl debug windows`), and the daemon's own state is what is checked afterwards, never bardo-settings's.
# Since porter's step 5c the pane reads bardo-accountd's module for every account (the identities: sign in again, remove, the
# apps using one; the AI keys: the Language model switch, the apps' permits with their Revoke, remove), bardo-inferd's for the
# spend rows beside them, and the sync service's bardo-syncd (not run here). The switch and the Revoke this scenario clicked on a mail account are
# the AI key's now.
# Real bardo-accountd, bardo-inferd, bardo-shell (the compositor's shell, and the sheet host), bardo-settings at its pin, the rig's fake servers.
#
#   boot      bardo-compositor, recorded bus, fakes, Secret Service, bardo-accountd, bardo-inferd, bardo-shell, bardo-settings (named org.bardo.Settings, role
#             `settings`, with the programs' schema fixtures; every daemon the pane reads runs first: bardo-settings never starts
#             one); a fake mail identity added through bardo-accountd's sheet, an app (org.bardo.Mail) says it uses it, an OpenAI
#             key added through the same sheet, an app (org.bardo.Companion) granted the key
#   open      Accounts in the sidebar, the AI key chosen in the list (capture of the list and of the account)
#   service   the Language model switch is turned off: bardo-accountd's `service.llm` row says so and the app that held the permit
#             is refused the key (ai-keys request: refused); turned on again, the key is back
#   keep      Remove account, then Keep: the key is still there (capture of the question)
#   revoke    Revoke on the app's row: the app's permit is gone
#   remove    Remove account, then Remove account in the question: the key is gone from bardo-accountd's module and registry, and
#             its audit file says removed; the same for the identity (a second removed in the audit)
#   added     an identity added through the sheet while bardo-settings is open shows in its pane (bardo-accountd sends Changed on an add)
#   sign-in   Sign in again on that identity: bardo-accountd opens the sign-in sheet (Settings reauthenticates any account)
#   scan      the secret scan
#
# budget: 300
# Exit: 0 pass | 1 a step failed | 2 outside the jail | 3 artefacts missing (harness/scenario-build.sh --accounts).
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../harness/lib/all.sh"
source "$(dirname "${BASH_SOURCE[0]}")/../harness/lib/scenario.sh"
source "$(dirname "${BASH_SOURCE[0]}")/../harness/lib/accounts.sh"
require_jail
acc_need
acc_env
# shellcheck source=/dev/null
source "$SCEN_SIB/shell/dev/lib/all.sh"
OUT_DIR=$(scenario_out accounts-settings)
IDENT=fake-mail-ada-rig.test   # the identity (bardo-accountd); AI is the OpenAI key's account (bardo-accountd's too), set at boot
AI=
AI_LABEL=
MAIL_LABEL=

cleanup() { acc_stop_all; stop_daemon; stop_compositor; scratch_drop; }
trap cleanup EXIT

app_has_no_permit() { [ -z "$(ai_grant_path org.bardo.Companion)" ]; }
service_llm() { ai_get "accounts.$AI.service.llm"; }
detent_mapped() { [ -n "$(detent_frame 2>/dev/null)" ]; }
no_ai() { ! ai_has_account "$AI"; }
has_ai() { ai_has_account "$AI"; }
no_ident() { ! has_ident; }
has_ident() { has_row_of settings_describe "$IDENT"; }
# The controls, by role and accessible name (bardo-settings's labels; detent_press, harness/lib/accounts.sh). The detail's
# buttons name the account or app they act on ("Remove account Home cloud"), so they are pressed by their lead
# ("Remove account ..."); a row of the list is named by the account and reads on after a comma ("Home cloud, OpenAI").
press_accounts() { detent_press option Accounts; }
choose_account() { detent_press option "$1 …"; }
# The service switch is named by the kind the module's row ends in (humanized: `llm` reads "Llm"), its row's label otherwise.
press_llm_switch() { if detent_centre switch Llm >/dev/null 2>&1; then detent_press switch Llm; else detent_press switch "Language model"; fi; }
press_revoke() { detent_press button "Revoke …"; }
# removed_events: how many `removed` events bardo-accountd's audit file holds (the key's removal, then the identity's).
removed_events() { grep -c '"kind":"removed"' "$STATE/bardo/accounts/audit.jsonl" 2>/dev/null; }
press_remove() { detent_press button "Remove account …"; }
press_sign_in_again() { detent_press button "Sign in again …"; }
# The question's buttons come after the page's own Remove account button in the document.
press_question_keep() { detent_press button Keep; }
press_question_remove() { detent_press button "Remove account …" -1; }

step_boot() {
  start_compositor || return 1
  scratch_new; acc_dirs
  build || return 1
  scan_begin
  rig_start --imap --smtp --tls implicit --llm-api || return 1
  secrets_start || return 1
  acc_config; acc_fake_mail_provider; acc_provider_openai_on_rig
  accountd_start || return 1
  inferd_write_config </dev/null
  inferd_start || return 1
  shell_start || return 1
  sheet_add_fake_mail || return 1
  IDENT=$ACCOUNT
  rigc org.bardo.Mail identities register-use "$(identity_of_account "$IDENT")" >"$OUT_DIR/use.log" 2>&1 \
    || { echo "register-use refused: $(cat "$OUT_DIR/use.log")" >&2; return 1; }
  sheet_add_openai || return 1
  AI=$ACCOUNT
  AI_LABEL=$(ai_get "accounts.$AI.label"); MAIL_LABEL=$(rigj imap.user)
  sheet_grant org.bardo.Companion grant || return 1
  detent_start
  acc_wait 30 detent_mapped || { echo "bardo-settings never mapped" >&2; tail -5 "$OUT_DIR/bardo-settings.log" >&2; return 1; }
  sleep 4
  detent_focus
}

step_open() {
  press_accounts; sleep 1; acc_shot 1-accounts-list
  choose_account "$AI_LABEL"; sleep 1; acc_shot 2-account
}

step_service() {
  local before after
  before=$(service_llm)
  press_llm_switch; sleep 1; acc_shot 3-llm-off
  after=$(service_llm)
  [ "$before" != "$after" ] || { echo "the Language model switch changed nothing in bardo-accountd (service.llm is still $before)" >&2; return 1; }
  ai_request_start org.bardo.Companion off; acc_wait 20 grep -q '"result"' "$OUT_DIR/off.log"
  grep -q '"result":"refused"' "$OUT_DIR/off.log" || { echo "with the switch off the app is still offered the key: $(cat "$OUT_DIR/off.log")" >&2; return 1; }
  press_llm_switch; sleep 1
  [ "$(service_llm)" = "$before" ] || { echo "turned on again, service.llm is $(service_llm), not $before" >&2; return 1; }
  # The app holds a permit (Always), so with the switch on again RequestKey answers it at once, no sheet.
  ai_request_start org.bardo.Companion on; acc_wait 20 grep -q '"result"' "$OUT_DIR/on.log"
  grep -q '"result":"granted"' "$OUT_DIR/on.log" || { echo "with the switch on again the app is not given its permit: $(cat "$OUT_DIR/on.log")" >&2; return 1; }
  sheet_hidden || { echo "a sheet was drawn for an app that holds a permit" >&2; sheet_stray_escape; return 1; }
}

step_revoke() {
  [ -n "$(ai_grant_path org.bardo.Companion)" ] || { echo "the app holds no permit to revoke (bardo-accountd lists none titled $(app_title org.bardo.Companion))" >&2; return 1; }
  press_revoke; sleep 1; acc_shot 4-revoked
  acc_wait 10 app_has_no_permit \
    || { echo "the app still has its permit: $(ai_grant_path org.bardo.Companion)" >&2; return 1; }
}

step_keep() {
  press_remove; sleep 1; acc_shot 5-remove-question
  press_question_keep; sleep 1
  has_ai || { echo "Keep removed the key" >&2; return 1; }
}

step_remove() {
  press_remove; sleep 1
  press_question_remove
  acc_wait 15 no_ai || { echo "the key is still in bardo-accountd's settings module after Remove" >&2; return 1; }
  sleep 1; acc_shot 6-removed
  grep -q "$AI" "$STATE/porter/registry.json" 2>/dev/null && { echo "the registry still holds the key" >&2; return 1; }
  [ "$(removed_events)" = 1 ] || { echo "the audit file has $(removed_events) removed events after the key was removed, not 1" >&2; return 1; }
  # And the identity, from its own page.
  choose_account "$MAIL_LABEL"; sleep 1
  press_remove; sleep 1
  press_question_remove
  acc_wait 15 no_ident || { echo "the identity is still in the settings module after Remove" >&2; return 1; }
  sleep 1; acc_shot 6-removed-identity
  grep -q "$IDENT" "$STATE/porter/registry.json" && { echo "the registry still holds the identity" >&2; return 1; }
  [ "$(removed_events)" = 2 ] || { echo "the audit file has $(removed_events) removed events after the identity was removed too, not 2" >&2; return 1; }
  return 0
}

# window_differs A B: the bardo-settings window of capture A differs from B's by more than 0.3% of its pixels (the rest of
# the output holds the bar's clock).
window_differs() {
  python3 -I - "$OUT_DIR/$1.png" "$OUT_DIR/$2.png" <<'PY'
import sys
from PIL import Image, ImageChops
box = (510, 198, 1410, 838)
a = Image.open(sys.argv[1]).convert("RGB").crop(box)
b = Image.open(sys.argv[2]).convert("RGB").crop(box)
diff = ImageChops.difference(a, b).convert("L").point(lambda v: 255 if v > 12 else 0)
share = sum(1 for v in diff.tobytes() if v) / (box[2] - box[0]) / (box[3] - box[1])
print("window differs by %.2f%%" % (share * 100), file=sys.stderr)
sys.exit(0 if share > 0.003 else 1)
PY
}

step_added_shows() {
  # A new identity, added while bardo-settings is open, must show in its Accounts pane (bardo-accountd tells the pane).
  sheet_add_fake_mail || return 1
  has_ident || { echo "bardo-accountd's settings module does not list the added identity" >&2; return 1; }
  sleep 4; acc_shot 7-after-add
  window_differs 6-removed-identity 7-after-add || {
    echo "bardo-settings still shows 'No accounts' 4 s after an identity was added: bardo-accountd sent no SettingsModule1.Changed for it, so the pane never re-reads" >&2
    return 1
  }
  # The page changing is not enough: an unavailable pane also differs from 'No accounts'. The added
  # identity's row must be there in the list.
  detent_centre option "$MAIL_LABEL …" >/dev/null || {
    echo "bardo-settings's Accounts pane changed after the add but shows no account row (it shows:)" >&2; detent_listing >&2
    return 1
  }
}

step_sign_in() {
  press_accounts; sleep 1; choose_account "$MAIL_LABEL"; sleep 1; acc_shot 8-account-again
  press_sign_in_again; sleep 2; acc_shot 9-after-sign-in-again
  if sheet_shown; then sheet_shot 10-sign-in-again; sheet_key Escape; return 0; fi
  echo "Sign in again opened no sheet: $(grep reauthenticate "$OUT_DIR/bardo-accountd.log" | tail -n 1)" >&2
  return 1
}

step_scan() { scan_end; }

run_steps step_boot step_open step_service step_keep step_revoke step_remove step_added_shows step_sign_in step_scan
if [ "$STATUS" != 0 ]; then
  for f in bardo-accountd bardo-inferd daemon bardo-settings rig-servers secrets add; do
    [ -f "$OUT_DIR/$f.log" ] && { echo "--- $f.log (tail)" >&2; tail -15 "$OUT_DIR/$f.log" >&2; }
  done
fi
exit "$STATUS"
