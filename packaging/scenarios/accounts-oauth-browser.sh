#!/usr/bin/env bash
# Scenario `accounts-oauth-browser` (design 31 §7.1): a Microsoft-shaped account is added through the browser
# sign-in against the sync service rig's fake OAuth issuer, the identity is stored, and Graph (the rig's fake drive, which takes only
# the access tokens the issuer minted) is reached by bardo-syncd (the sync service's), which has no accounts of its own: it uses bardo-accountd's
# account through account-client over org.bardo.Grants1, after one consent sheet for that account.
#
# The browser is the scenario's own `xdg-open` first on bardo-shell's PATH (harness/lib/accounts.sh shell_start): bardo-shell opens
# the page the sheet shows (BrowserWait), the opener records the address and follows it with curl, redirects
# included, so the issuer's authorize page answers with a redirect to bardo-accountd's loopback listener, which takes the
# code. The person's consent is the issuer's default (grant); the Microsoft client and its endpoints are the
# shipped clients.toml (the jail's /usr/share overlay), Graph is the shipped microsoft.toml with its host pointed at the fake.
#
#   boot      bardo-compositor, recorded bus, the sync service's rig (issuer + Graph), fake Secret Service, bardo-accountd, bardo-shell
#   sign-in   AddAccount, "Micro" filters, Return picks Microsoft: the sheet shows the browser wait (capture); bardo-shell
#             opens the issuer's authorize URL (PKCE S256 and the loopback redirect); the sign-in finishes on its own
#             and the review step shows (capture); Return is Done; add-account answers `added`
#   stored    the account is `ok` in bardo-accountd's settings module, signed_in is in the audit file, the account is in
#             the registry (and, by the scan below, no token is in any file)
#   graph     the files switch of the account in bardo-syncd's rows is turned on (accounts.<id>.sync.files, as Settings sets it): bardo-accountd's
#             consent sheet shows once, for that one account (capture, Always Allow; no chooser, no second sign-in), and bardo-syncd reaches
#             Graph with the issuer's access token (GET /v1.0/me/drive...: the sync service rig's hits lever lists a request with a
#             bearer); bardo-accountd lists bardo-syncd's permit (org.bardo.Sync, always) under the account
#   scan      the secret scan
#
# budget: 240
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
OUT_DIR=$(scenario_out accounts-oauth-browser)

cleanup() { acc_stop_all; stop_daemon; stop_compositor; scratch_drop; }
trap cleanup EXIT

step_boot() {
  start_compositor || return 1
  scratch_new; acc_dirs
  build || return 1
  scan_begin
  sync_rig_start || return 1
  secrets_start || return 1
  acc_config; acc_clients_for_issuer; acc_provider_microsoft_on_rig
  accountd_start || return 1
  shell_start || return 1
  syncd_start || return 1
}

step_sign_in() {
  acc_add_start org.bardo.Mail
  sheet_await || return 1; sleep 1
  sheet_settled; sheet_type Micro; sleep 0.5; sheet_key Enter
  acc_wait 20 test -s "$RIG_DIR/opened.log" || { echo "bardo-shell never opened the browser: $(cat "$OUT_DIR/add.log")" >&2; return 1; }
  sheet_shot 1-browser-wait
  grep -q "^$(frigj oauth.authorize)" "$RIG_DIR/opened.log" || { echo "bardo-shell opened $(cat "$RIG_DIR/opened.log"), not the issuer's authorize page" >&2; return 1; }
  grep -q 'code_challenge_method=S256' "$RIG_DIR/opened.log" || { echo "the authorize URL has no PKCE S256" >&2; return 1; }
  # The browser followed the redirect to bardo-accountd's loopback listener: the review step comes up on its own.
  sleep 3; sheet_shot 2-review
  frigctl GET /graph/hits >"$OUT_DIR/graph-hits.json"
  sheet_key Enter
  local response
  response=$(acc_add_response 30) || { echo "AddAccount never answered: $(cat "$OUT_DIR/add.log")" >&2; return 1; }
  acc_added "$response" || { echo "AddAccount answered $response" >&2; return 1; }
  sheet_gone || return 1
  echo "added $ACCOUNT" >&2
}

step_stored() {
  [ "$(settings_get "accounts.$ACCOUNT.state")" = ok ] || { echo "state is $(settings_get "accounts.$ACCOUNT.state")" >&2; return 1; }
  grep -q '"kind":"signed_in"' "$STATE/bardo/accounts/audit.jsonl" || { echo "no signed_in in the audit file" >&2; return 1; }
  # The refresh token went to the Secret Service (the scan lists that call as the one place it may go) and into no
  # file: the registry holds the account, never a credential.
  grep -q "$ACCOUNT" "$STATE/porter/registry.json" || { echo "the account is not in the registry" >&2; return 1; }
}

graph_bearer() { frigctl GET /graph/hits | grep -q '"bearer":true'; }

step_graph() {
  # bardo-syncd lists bardo-accountd's account by its own id; its files switch is the first use: bardo-accountd asks once.
  acc_wait 30 sync_has_account "$ACCOUNT" || { echo "bardo-syncd lists no row for bardo-accountd's account $ACCOUNT: $(tail -5 "$OUT_DIR/bardo-syncd.log")" >&2; return 1; }
  sync_files_turn_on "$ACCOUNT"
  sheet_await || { echo "bardo-accountd's consent sheet for bardo-syncd never showed: $(tail -5 "$OUT_DIR/bardo-syncd.log")" >&2; return 1; }
  sheet_settled; sheet_shot 3-consent
  sync_consent_keys
  acc_wait 90 graph_bearer || { echo "Graph saw no request with the issuer's token from bardo-syncd: $(frigctl GET /graph/hits)" >&2; tail -10 "$OUT_DIR/bardo-syncd.log" >&2; return 1; }
  # bardo-accountd lists the permit under the account (`accounts.<id>.grant.<grant>.app`); `used_by` is Identities1.RegisterUse's list, which
  # an app makes when it signs in with an identity, and bardo-syncd holds a grant instead.
  acc_wait 20 eval '[ -n "$(acc_permit_ids "$ACCOUNT" $SYNC_APP)" ]' || { echo "bardo-accountd lists no permit of $SYNC_APP on the account after bardo-syncd was allowed to use it" >&2; return 1; }
  [ "$(acc_permit_scope "$ACCOUNT" $SYNC_APP)" = always ] || { echo "bardo-syncd's permit is '$(acc_permit_scope "$ACCOUNT" $SYNC_APP)', not always" >&2; return 1; }
  echo "graph: $(frigctl GET /graph/hits)" >&2
}

step_scan() { scan_end; }

run_steps step_boot step_sign_in step_stored step_graph step_scan
if [ "$STATUS" != 0 ]; then
  for f in bardo-accountd bardo-syncd daemon rig-servers sync-rig-servers secrets add; do
    [ -f "$OUT_DIR/$f.log" ] && { echo "--- $f.log (tail)" >&2; tail -15 "$OUT_DIR/$f.log" >&2; }
  done
  [ -f "$RIG_DIR/browser.log" ] && { echo "--- browser.log" >&2; tail -10 "$RIG_DIR/browser.log" >&2; }
fi
exit "$STATUS"
