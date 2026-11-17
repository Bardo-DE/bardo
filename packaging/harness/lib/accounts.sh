# What the accounts-* scenarios share: porter's real bardo-accountd, the sync service's bardo-syncd, the inference service's bardo-inferd, porter's test rig
# (account-rig-servers: the fakes, account-rig-secrets: a fake Secret Service, account-rig-client: an app), the sync service's test rig
# (sync-rig-servers: the OAuth issuer and the Graph fake, sync-rig-client: an app talking to bardo-syncd) and inference-rig-client (an app opening an bardo-inferd session) on the
# private bus bardo-compositor's dev/up.sh made, with bardo-shell drawing the sheets. Sourced after harness/lib/all.sh,
# harness/lib/scenario.sh and bardo-shell's dev/lib/all.sh (it uses their in_env, bardo-shell, surface_named, capture ...).
#
# Identity. The daemons (bardo-inferd is the inference service's, from build/inference-target) are test-proc-root builds: they name the process behind a bus connection from
# $PROC/<pid>/cgroup (BARDO_ACCOUNTD_PROC_ROOT, BARDO_SYNCD_PROC_ROOT, BARDO_INFERD_PROC_ROOT; bardo-syncd is the sync service's, from build/sync-target), because nothing in the jail runs
# in a systemd scope; the role of each is the system caller table, porter's dist copy at $ACC_SYSTEM_CALLERS
# (BARDO_ACCOUNTD_SYSTEM_CALLERS: the jail cannot write /etc/bardo/accountd-callers.toml, and a person's own file gives no fixed role);
# bardo-inferd's is $BARDO_INFERD_SYSTEM_CALLERS (BARDO_INFERD_CALLERS, in place of /etc/bardo/inferd-callers.toml: porter
# reads no [callers] from bardo-inferd.toml since CALLER-INFERD; the inference service's dist/inferd-callers.toml is its shipped copy).
# `acc_identity UNIT PID` writes a unit's cgroup (bardo-shell.service, bardo-syncd.service, bardo-inferd.service) and, for a service
# unit, its main pid ($PROC/units/<unit>); `acc_app APP PID` names bardo-settings the Settings unit (org.bardo.Settings.service)
# and any other app a Flatpak (its scope and $PROC/<pid>/root/.flatpak-info).
#
# Every process is stopped by its PID (acc_stop_all); nothing is found by name.

ACC_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ARIG="$SCEN_BUILD/porter-target/debug"
ASTW="$SCEN_BUILD/inference-target/debug"
AFER="$SCEN_BUILD/sync-target/debug"
ACC_PIDS=()
ACC_BG=()

# acc_need: the artefacts exist (exit 3 with the fix otherwise).
acc_need() {
  scenario_need "$SCEN_TOOLS/bin/bardo-compositor" "$SCEN_TOOLS/bin/bardo-compositorctl" "$SCEN_TOOLS/bin/fake-logind" \
    "$SCEN_BUILD/shell-probe-target/debug/bardo-shell" "$ARIG/bardo-accountd" "$AFER/bardo-syncd" "$ASTW/bardo-inferd" \
    "$ARIG/account-rig-servers" "$ARIG/account-rig-client" "$ARIG/account-rig-secrets" "$ASTW/inference-rig-client" \
    "$AFER/sync-rig-servers" "$AFER/sync-rig-client" \
    "$SCEN_SIB/accounts/providers/microsoft.toml" "$SCEN_SIB/accounts/dist/callers.toml" "$@"
}

# acc_env: the variables every accounts scenario sets before sourcing bardo-shell's lib (BARDO_COMPOSITOR_REPO and BARDO_COMPOSITOR_DIR
# become the stand-in checkout floor_shim makes).
acc_env() {
  export BARDO_SHELL_COMPOSITOR=bardo BARDO_SHELL_PREBUILT=1 BARDO_SHELL_PROFILE=debug
  export CARGO_TARGET_DIR="$SCEN_BUILD/shell-probe-target" BARDO_COMPOSITOR_TOOLS="$SCEN_TOOLS"
  # The rig's programs take no power; bardo-compositorctl's `shot` (acc_shot) takes the screen and the window list, so those are the
  # floor's only rows and nothing waits on a question (an unanswered ask stops the capture after ~29 s, and its dialog
  # covers the page). The caller list names bardo-shell (at its pin) as the shell, bardo-settings as Settings and the porter daemons by
  # their roles, for bardo-compositor and for bardo-shell (floor_shell_callers before launch_daemon).
  FLOOR_SHELL_TARGET="$SCEN_BUILD/shell-probe-target" floor_shim accounts bardo-shell bardo-settings bardo-accountd bardo-inferd bardo-syncd bardo-compositor bardo-compositorctl || return 1
}

# Where the scenario keeps things; set by acc_dirs after scratch_new.
RIG_DIR= PROC= PROV= BIN_DIR= ACC_SYSTEM_CALLERS= BARDO_INFERD_SYSTEM_CALLERS=
acc_dirs() {
  RIG_DIR="$OUT_DIR/rig"; PROC="$RIG_DIR/proc"; PROV="$RIG_DIR/providers"; BIN_DIR="$RIG_DIR/bin"
  ACC_SYSTEM_CALLERS="$RIG_DIR/system-callers.toml"
  BARDO_INFERD_SYSTEM_CALLERS="$RIG_DIR/inferd-callers.toml"
  mkdir -p "$RIG_DIR" "$PROC" "$PROV" "$BIN_DIR" "$CONFIG/bardo/accounts" "$CONFIG/bardo" "$STATE/bardo/accounts"
}

# acc_bg LOG CMD...: runs CMD in the background in the daemon environment with this log; sets ACC_LAST to its pid.
ACC_LAST=
acc_bg() {
  local log=$1; shift
  in_env "$@" >"$log" 2>&1 &
  ACC_LAST=$!
  ACC_PIDS+=("$ACC_LAST")
}

# acc_kill PID: ends one process we started, by its PID, and waits up to 5 s.
acc_kill() {
  kill "$1" 2>/dev/null || return 0
  for _ in $(seq 50); do kill -0 "$1" 2>/dev/null || { wait "$1" 2>/dev/null; return 0; }; sleep 0.1; done
  kill -9 "$1" 2>/dev/null; wait "$1" 2>/dev/null
}

acc_stop_all() {
  local pid
  for pid in "${ACC_PIDS[@]:-}"; do [ -z "$pid" ] || acc_kill "$pid"; done
  ACC_PIDS=()
}

# acc_wait SECONDS CMD...: polls CMD (every 0.1 s) until it succeeds.
acc_wait() {
  # By the clock, not by tries: a check that itself takes seconds (a SyncNow cycle) must not stretch the wait.
  local end=$(( SECONDS + $1 )); shift
  while :; do
    "$@" >/dev/null 2>&1 && return 0
    [ "$SECONDS" -lt "$end" ] || return 1
    sleep 0.1
  done
}

# bus_has_owner NAME: the name has an owner on the private bus.
bus_has_owner() {
  dbus-send --bus="$BUS" --print-reply --dest=org.freedesktop.DBus /org/freedesktop/DBus \
    org.freedesktop.DBus.NameHasOwner "string:$1" 2>/dev/null | grep -q 'boolean true'
}

# --- identity ------------------------------------------------------------------------------------------------
# The Settings app is known by its unit: bardo-settings running as org.bardo.Settings.service and being that unit's
# main process (bardo-accountd asks systemd; with a fixture proc root it reads $PROC/units/<unit>).
SETTINGS_UNIT=org.bardo.Settings.service

acc_identity() { # LEAF PID: a unit's cgroup; a service unit also gets its main pid ($PROC/units/<unit>)
  mkdir -p "$PROC/$2"
  printf '0::/user.slice/user-1000.slice/user@1000.service/app.slice/%s\n' "$1" >"$PROC/$2/cgroup"
  case "$1" in
    *.service) mkdir -p "$PROC/units"; printf '%s\n' "$2" >"$PROC/units/$1" ;;
  esac
}
# acc_flatpak APP-ID PID: a Flatpak app: its scope, and the sandbox's own metadata, which is what names the app.
acc_flatpak() {
  local esc=${1//-/\\x2d}
  mkdir -p "$PROC/$2/root"
  printf '0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-%s-%s.scope\n' "$esc" "$2" >"$PROC/$2/cgroup"
  printf '[Application]\nname=%s\n' "$1" >"$PROC/$2/root/.flatpak-info"
}
acc_app() { # APP-ID PID: Settings is its service unit, any other app a Flatpak
  case "$1" in
    org.bardo.Settings) acc_identity "$SETTINGS_UNIT" "$2" ;;
    *) acc_flatpak "$1" "$2" ;;
  esac
}

# --- the fakes -----------------------------------------------------------------------------------------------
# rig_start FLAGS...: account-rig-servers --dir $RIG_DIR/servers FLAGS; waits for rig.json.
RIG_PID=
BARDO_SYNC_RIG_PID=
rig_start() {
  mkdir -p "$RIG_DIR/servers"
  acc_bg "$OUT_DIR/rig-servers.log" "$ARIG/account-rig-servers" --dir "$RIG_DIR/servers" "$@"
  RIG_PID=$ACC_LAST
  acc_wait 20 test -s "$RIG_DIR/servers/rig.json" || { echo "account-rig-servers never wrote rig.json:" >&2; cat "$OUT_DIR/rig-servers.log" >&2; return 1; }
}
# sync_rig_start [FLAGS...]: sync-rig-servers --dir $RIG_DIR/sync FLAGS (--token-lifetime-s N); it always starts the OAuth
# issuer and the Graph fake, and writes its own rig.json. It runs beside account-rig-servers (rig_start) when a scenario
# needs both; frigj and frigctl read and drive this one.
sync_rig_start() {
  mkdir -p "$RIG_DIR/sync"
  acc_bg "$OUT_DIR/sync-rig-servers.log" "$AFER/sync-rig-servers" --dir "$RIG_DIR/sync" "$@"
  BARDO_SYNC_RIG_PID=$ACC_LAST
  acc_wait 20 test -s "$RIG_DIR/sync/rig.json" || { echo "sync-rig-servers never wrote rig.json:" >&2; cat "$OUT_DIR/sync-rig-servers.log" >&2; return 1; }
}
# rigj PATH: a value of rig.json (a dotted path, `imap.port`), or the planted secrets with `secrets`. It reads porter's rig
# (RIG_JSON_DIR, default $RIG_DIR/servers); frigj reads the sync service's (oauth.*, graph.url, secrets).
rigj() {
  python3 -I -c '
import json, sys
v = json.load(open(sys.argv[1]))
for k in sys.argv[2].split("."): v = v[k]
print("\n".join(v) if isinstance(v, list) else v)' "${RIG_JSON_DIR:-$RIG_DIR/servers}/rig.json" "$1"
}
frigj() { RIG_JSON_DIR="$RIG_DIR/sync" rigj "$@"; }
# rigctl METHOD TARGET [BODY]: a lever of porter's rig control endpoint; the JSON answer. frigctl: the same on the sync service's rig
# (/graph/..., /issuer/...).
rigctl() {
  local ctl; ctl=$(rigj control) || return 1
  curl -sS -X "$1" --data-binary "${3:-}" "$ctl$2"
}
frigctl() { RIG_JSON_DIR="$RIG_DIR/sync" rigctl "$@"; }

secrets_start() {
  acc_bg "$OUT_DIR/secrets.log" "$ARIG/account-rig-secrets"
  SECRETS_PID=$ACC_LAST
  acc_wait 20 bus_has_owner org.freedesktop.secrets || { echo "the fake Secret Service never took its name" >&2; cat "$OUT_DIR/secrets.log" >&2; return 1; }
}

# --- configuration -------------------------------------------------------------------------------------------
# acc_config: the system caller table (porter's own, read through BARDO_ACCOUNTD_SYSTEM_CALLERS), the shipped provider
# files, and the person's clients file (acc_clients_for_issuer writes the shipped one).
acc_config() {
  cp "$SCEN_SIB/accounts/dist/callers.toml" "$ACC_SYSTEM_CALLERS"
  cp "$SCEN_SIB"/accounts/providers/*.toml "$PROV/"
}

# acc_clients_for_issuer: the shipped clients.toml (/usr/share/bardo/accounts, the jail's throwaway overlay) pointing the
# Microsoft client at the fake issuer of the sync service's rig (the one its Graph fake accepts tokens from; sync_rig_start first): porter takes endpoints that leave the issuer only from the shipped file, never
# from the person's own (sec-4).
acc_clients_for_issuer() {
  mkdir -p /usr/share/bardo/accounts
  cat >/usr/share/bardo/accounts/clients.toml <<EOF
[[client]]
issuer = "microsoft"
channel = "stable"
client_id = "$(frigj oauth.client_id)"
[client.endpoints]
authorize = "$(frigj oauth.authorize)"
token = "$(frigj oauth.token)"
revoke = "$(frigj oauth.revoke)"
device = "$(frigj oauth.device)"
EOF
}

# acc_provider_microsoft_on_rig: the shipped microsoft.toml with Graph at the sync service's rig drive.
acc_provider_microsoft_on_rig() {
  sed -e "s#https://graph.microsoft.com#$(frigj graph.url)#g" "$SCEN_SIB/accounts/providers/microsoft.toml" >"$PROV/microsoft.toml"
}

# acc_providers_only ID...: leaves only the named provider files (so the sheet's list is short).
acc_providers_only() {
  local keep=" $* " f id
  for f in "$PROV"/*.toml; do
    id=$(basename "$f" .toml)
    case "$keep" in *" $id "*) ;; *) rm -f "$f" ;; esac
  done
}

# --- the daemons ---------------------------------------------------------------------------------------------
# Since porter's step 5c bardo-accountd holds every account: the identities, the AI keys with their permits and switches (its
# Keys interface), and the local runtimes bardo-inferd's probe reports to it (Broker.ReportLocal, a call only the `ai_broker`
# role may make: bardo-inferd.service in dist/callers.toml). bardo-inferd (org.bardo.Inference1) holds no key; it serves the models
# and the spend rows (`accounts.<id>.spend.*`) of the accounts bardo-accountd lists. bardo-syncd (the sync service's) has no accounts of
# its own: it uses bardo-accountd's through account-client over org.bardo.Grants1. A page that reads a daemon never starts it (bardo-settings's live/io.rs answers NotRunning), so a scenario
# starts every daemon its page reads before opening the page.
# Each daemon reads its caller table at startup only: acc_config installs callers.toml before bardo-accountd and bardo-syncd start,
# inferd_write_config writes bardo-inferd's own system table ($BARDO_INFERD_SYSTEM_CALLERS) before bardo-inferd starts.

# acc_ca_env: SSL_CERT_FILE=<the rig's CA> when the rig made one, else nothing (one word per line, for `env`).
acc_ca_env() { [ ! -f "$RIG_DIR/servers/ca.pem" ] || echo "SSL_CERT_FILE=$RIG_DIR/servers/ca.pem"; }

# acc_ship_providers: the provider files of $PROV (the pin's, with the rig's brand and fake-host files) into the jail's
# /usr/share overlay, the Shipped layer. bardo-inferd and bardo-syncd have no --providers: they read the first XDG_DATA_DIRS entry
# that has porter/providers (the jail pins XDG_DATA_DIRS to /usr/share, so a host install under /usr/local/share
# cannot shadow it) and $XDG_DATA_HOME/bardo/accounts/providers, and a person's layer may not point a shipped provider at
# another server.
acc_ship_providers() {
  mkdir -p /usr/share/bardo/accounts/providers
  cp "$PROV"/*.toml /usr/share/bardo/accounts/providers/
}

# accountd_start: the real bardo-accountd on the private bus, over the fake Secret Service, trusting the rig's CA.
BARDO_ACCOUNTD_PID=
accountd_start() {
  local ca; ca=$(acc_ca_env)
  acc_bg "$OUT_DIR/bardo-accountd.log" env BARDO_ACCOUNTD_PROC_ROOT="$PROC" BARDO_ACCOUNTD_SYSTEM_CALLERS="$ACC_SYSTEM_CALLERS" ${ca:+"$ca"} "$ARIG/bardo-accountd" --providers "$PROV"
  BARDO_ACCOUNTD_PID=$ACC_LAST
  acc_wait 20 bus_has_owner org.bardo.Accounts1 || { echo "bardo-accountd never took org.bardo.Accounts1:" >&2; cat "$OUT_DIR/bardo-accountd.log" >&2; return 1; }
  acc_identity bardo-accountd.service "$BARDO_ACCOUNTD_PID"
}
accountd_stop() { [ -z "$BARDO_ACCOUNTD_PID" ] || acc_kill "$BARDO_ACCOUNTD_PID"; BARDO_ACCOUNTD_PID=; }

BARDO_SYNCD_PID=
syncd_start() {
  local ca; ca=$(acc_ca_env)
  acc_ship_providers
  acc_bg "$OUT_DIR/bardo-syncd.log" env BARDO_SYNCD_PROC_ROOT="$PROC" BARDO_ACCOUNTD_SYSTEM_CALLERS="$ACC_SYSTEM_CALLERS" BARDO_SYNCD_RESCAN_S="${BARDO_SYNCD_RESCAN_S:-2}" ${ca:+"$ca"} "$AFER/bardo-syncd"
  BARDO_SYNCD_PID=$ACC_LAST
  acc_identity bardo-syncd.service "$BARDO_SYNCD_PID"
  acc_wait 20 bus_has_owner org.bardo.Sync1 || { echo "bardo-syncd never took org.bardo.Sync1:" >&2; cat "$OUT_DIR/bardo-syncd.log" >&2; return 1; }
}
syncd_stop() { [ -z "$BARDO_SYNCD_PID" ] || acc_kill "$BARDO_SYNCD_PID"; BARDO_SYNCD_PID=; }

# inferd_write_config: $CONFIG/bardo/inference/inferd.toml is what is on stdin (tables of the scenario: [probe], [ai] ...), and
# $BARDO_INFERD_SYSTEM_CALLERS is bardo-inferd's caller table, in the shape of porter's dist/inferd-callers.toml: bardo-inferd serves its
# settings module (the spend rows) to the unit org.bardo.Settings.service and its places to the shell (bardo-shell.service); an
# app in a Flatpak or app scope needs no row. The file is bardo-inferd's own folder's, where Settings looks for it. Call it
# before inferd_start.
inferd_write_config() {
  mkdir -p "$CONFIG/bardo/inference"
  cat >"$CONFIG/bardo/inference/inferd.toml"
  printf 'settings = ["%s"]\nshell = ["bardo-shell.service"]\n' "$SETTINGS_UNIT" >"$BARDO_INFERD_SYSTEM_CALLERS"
}

BARDO_INFERD_PID=
inferd_start() { # needs $CONFIG/bardo/inference/inferd.toml and $BARDO_INFERD_SYSTEM_CALLERS (inferd_write_config)
  # The model catalog of this tree's model layer, where bardo-inferd reads it ($XDG_DATA_HOME/bardo/models/catalog): without
  # it no hosted model is known, so a turn on an AI key is refused before the key is ever read.
  mkdir -p "$DATA/bardo/models/catalog"
  cp "$SCEN_SIB"/models/catalog/*.toml "$DATA/bardo/models/catalog/"
  local ca; ca=$(acc_ca_env)
  acc_ship_providers
  acc_bg "$OUT_DIR/bardo-inferd.log" env BARDO_INFERD_PROC_ROOT="$PROC" BARDO_INFERD_CALLERS="$BARDO_INFERD_SYSTEM_CALLERS" ${ca:+"$ca"} "$ASTW/bardo-inferd"
  BARDO_INFERD_PID=$ACC_LAST
  acc_identity bardo-inferd.service "$BARDO_INFERD_PID"
  acc_wait 20 bus_has_owner org.bardo.Inference1 || { echo "bardo-inferd never took org.bardo.Inference1:" >&2; cat "$OUT_DIR/bardo-inferd.log" >&2; return 1; }
}

# shell_start: bardo-shell (the sheet host) on the same bus; its pid is named bardo-shell.service (and is that unit's main pid), the sheet host's unit.
shell_start() {
  cat >"$BIN_DIR/xdg-open" <<EOF
#!/usr/bin/env bash
# The scenario's browser: records the address, then follows it (redirects included) the way a person's
# browser would; the issuer's consent is a lever of the rig.
echo "\$1" >>"$RIG_DIR/opened.log"
curl -sL --max-time 10 -o "$RIG_DIR/browser-last.html" "\$1" >>"$RIG_DIR/browser.log" 2>&1
EOF
  chmod +x "$BIN_DIR/xdg-open"
  mkdir -p "$CONFIG/bardo/shell"; : >"$CONFIG/bardo/shell/settings.toml"
  backdrop_seed "$CONFIG" || return 1
  # bardo-shell is built with ui-blitz's debug-probe (scenario-build.sh): every window of it lists its controls in $SPROBE.
  SPROBE="$SCRATCH/bardo-shell-probe"; mkdir -p "$SPROBE"
  IN_ENV_EXTRA=("PATH=$BIN_DIR:$PATH" "BARDO_DEBUG_PROBE=$SPROBE")
  # bardo-shell names itself to bardo-accountd within its first moments (its accounts service asks NeedingReauth as it connects, and a
  # refused first ask is not repeated), so its identity is written by the shell that execs it, before it exists.
  LAUNCH_PREFIX=(bash -c 'mkdir -p "$0/$$" "$0/units"; echo "0::/user.slice/user-1000.slice/user@1000.service/app.slice/bardo-shell.service" >"$0/$$/cgroup"; echo "$$" >"$0/units/bardo-shell.service"; exec "$@"' "$PROC")
  floor_shell_callers
  launch_daemon || return 1
  NAME=$(output_name) || return 1
  SHEET="bardo-shell-accounts-sheet-$NAME"
}

# --- an app --------------------------------------------------------------------------------------------------
# rigc APP-ID ARGS...: account-rig-client as that app (its identity is written before it connects).
rigc() {
  local app=$1; shift
  in_env "$ARIG/account-rig-client" --app-id "$app" --proc-root "$PROC" "$@"
}
# frigc APP-ID ARGS...: sync-rig-client as that app (datasets, status, sync-resolve, watch-conflicts; same identity fixture).
frigc() {
  local app=$1; shift
  in_env "$AFER/sync-rig-client" --app-id "$app" --proc-root "$PROC" "$@"
}
# frigc_bg LOG APP-ID ARGS...: the same in the background; sets ACC_LAST to its pid.
frigc_bg() {
  local log=$1 app=$2; shift 2
  acc_bg "$log" "$AFER/sync-rig-client" --app-id "$app" --proc-root "$PROC" "$@"
}
# rigc_bg LOG APP-ID ARGS...: the same in the background; sets ACC_LAST to its pid.
rigc_bg() {
  local log=$1 app=$2; shift 2
  acc_bg "$log" "$ARIG/account-rig-client" --app-id "$app" --proc-root "$PROC" "$@"
}
# rigo APP-ID NEED ARGS...: inference-rig-client as that app, opening an bardo-inferd session (the same identity fixture).
rigo() {
  local app=$1; shift
  in_env "$ASTW/inference-rig-client" --app-id "$app" --proc-root "$PROC" open "$@"
}

# --- the sheet, as bardo-shell draws it -----------------------------------------------------------------------------
# sheet_shown: the sheet's surface draws something now.
sheet_shown() {
  bardo-shell debug capture "$SHEET" "$OUT_DIR/.probe.png" >/dev/null 2>&1 && [ "$(opaque_pixels "$OUT_DIR/.probe.png")" -gt 1000 ]
}
sheet_hidden() { ! sheet_shown; }
sheet_await() { acc_wait "${1:-15}" sheet_shown || { echo "the sheet never showed" >&2; return 1; }; }
sheet_gone() { acc_wait "${1:-15}" sheet_hidden || { echo "the sheet never went" >&2; return 1; }; }
# sheet_shot NAME: a capture of the sheet into the scenario's output dir.
sheet_shot() { bardo-shell debug capture "$SHEET" "$OUT_DIR/$1.png" >>"$OUT_DIR/capture.log" 2>&1; }
sheet_key() { bardo-shell debug key "$SHEET" "$1" >>"$OUT_DIR/keys.log" 2>&1; }
# sheet_settle [SECONDS]: wait until two captures 0.2 s apart are the same picture, so what was typed has been
# drawn (the list filtered) before the next key. A fixed sleep after typing flaked under a loaded gate.
sheet_settle() {
  local until=$(( SECONDS + ${1:-8} )) a="$OUT_DIR/.settle-a.png" b="$OUT_DIR/.settle-b.png"
  bardo-shell debug capture "$SHEET" "$a" >/dev/null 2>&1 || return 0
  while (( SECONDS < until )); do
    sleep 0.2
    bardo-shell debug capture "$SHEET" "$b" >/dev/null 2>&1 || return 0
    cmp -s "$a" "$b" && return 0
    mv -f "$b" "$a"
  done
  echo "the sheet kept changing" >&2
}
# sheet_type TEXT: one `bardo-shell debug key` per character (layer-shell-host e411b25 types the US shifted symbols, `@` among them, with
# Shift held; the pin has it).
sheet_type() {
  local text=$1 i c
  for (( i = 0; i < ${#text}; i++ )); do
    c=${text:i:1}
    case "$c" in " ") c=Space ;; esac
    bardo-shell debug key "$SHEET" "$c" >>"$OUT_DIR/keys.log" 2>&1 || { echo "bardo-shell debug key refused '$c'" >&2; return 1; }
  done
}

# --- a record of the bus -------------------------------------------------------------------------------------
MONITOR_PID=
monitor_start() {
  busctl --address="$BUS" monitor --json=short >"$OUT_DIR/bus.jsonl" 2>"$OUT_DIR/bus-monitor.err" &
  MONITOR_PID=$!
  ACC_PIDS+=("$MONITOR_PID")
}
monitor_stop() { [ -z "$MONITOR_PID" ] || acc_kill "$MONITOR_PID"; MONITOR_PID=; }

# --- asking bardo-accountd as an app, with nothing but busctl ------------------------------------------------------
# as_app APP-ID CMD...: runs CMD as a process that bardo-accountd names APP-ID (written for the pid that becomes CMD, so the pid
# the bus reports is CMD's). Any app but Settings is a Flatpak. Settings is its service unit's main process: while CMD
# runs, the unit's pid file ($PROC/units/<unit>) holds CMD's pid, and the previous pid (bardo-settings's) is put back after, so
# bardo-settings itself is refused for the length of the call and not after.
as_app() {
  local app=$1; shift
  # shellcheck disable=SC2016  # the script is for the inner bash, which expands it
  in_env bash -c '
    app=$1; proc=$2; unit=$3; shift 3
    if [ "$app" = org.bardo.Settings ]; then
      prev=$(cat "$proc/units/$unit" 2>/dev/null) || prev=
      bash -c '"'"'
        proc=$1; unit=$2; shift 2
        mkdir -p "$proc/$$" "$proc/units"
        printf "0::/user.slice/user-1000.slice/user@1000.service/app.slice/%s\n" "$unit" >"$proc/$$/cgroup"
        printf "%s\n" "$$" >"$proc/units/$unit"
        exec "$@"'"'"' _ "$proc" "$unit" "$@"
      status=$?
      [ -z "$prev" ] || printf "%s\n" "$prev" >"$proc/units/$unit"
      exit "$status"
    fi
    esc=$(printf %s "$app" | sed "s/-/\\\\x2d/g")
    mkdir -p "$proc/$$/root"
    printf "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-%s-$$.scope\n" "$esc" >"$proc/$$/cgroup"
    printf "[Application]\nname=%s\n" "$app" >"$proc/$$/root/.flatpak-info"
    exec "$@"' _ "$app" "$PROC" "$SETTINGS_UNIT" "$@"
}

# settings_describe_at BUS PATH / settings_get_at BUS PATH KEY: a daemon's settings module (org.bardo.SettingsModule1), read
# as the Settings app. Three daemons serve one: bardo-accountd (every account: identities, AI keys with their permits and
# switches, local runtimes; /org/bardo/Accounts1/settings), bardo-inferd (only the spend rows, `accounts.<id>.spend.*`, of the
# accounts bardo-accountd lists; /org/bardo/Inference1/settings) and bardo-syncd (the sync service's: what it keeps of bardo-accountd's accounts, /org/bardo/Sync1/settings).
settings_describe_at() {
  as_app org.bardo.Settings busctl --address="$BUS" --json=short call "$1" "$2" \
    org.bardo.SettingsModule1 Describe | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["data"][0])'
}
settings_get_at() {
  as_app org.bardo.Settings busctl --address="$BUS" --json=short call "$1" "$2" \
    org.bardo.SettingsModule1 Get s "$3" | python3 -I -c '
import json, sys
d = json.load(sys.stdin)["data"][0]
while isinstance(d, dict) and "data" in d: d = d["data"]
print(d)'
}
# settings_press_at BUS PATH KEY: presses an action row (Set with a false value), as the Settings app.
settings_press_at() {
  as_app org.bardo.Settings busctl --address="$BUS" --json=short call "$1" "$2" \
    org.bardo.SettingsModule1 Set sv "$3" b false >/dev/null
}
ACC_SET=/org/bardo/Accounts1/settings
SYNC_SET=/org/bardo/Sync1/settings
settings_describe() { settings_describe_at org.bardo.Accounts1 $ACC_SET; }
settings_get() { settings_get_at org.bardo.Accounts1 $ACC_SET "$1"; }
# The AI accounts (keys, permits, switches, local runtimes) are rows of bardo-accountd's module beside the identities'.
ai_describe() { settings_describe; }
ai_get() { settings_get "$1"; }
sync_describe() { settings_describe_at org.bardo.Sync1 $SYNC_SET; }
sync_get() { settings_get_at org.bardo.Sync1 $SYNC_SET "$1"; }
sync_press() { settings_press_at org.bardo.Sync1 $SYNC_SET "$1"; }
# acc_press KEY: an action row of bardo-accountd's module (Internet Accounts owns Sign in again: accounts.<id>.reauth).
acc_press() { settings_press_at org.bardo.Accounts1 $ACC_SET "$1"; }

# ai_has_account ID / has_key MODULE ID: the module lists an `accounts.<id>.` row.
has_row_of() { # DESCRIBE-FN ID
  "$1" 2>/dev/null | python3 -I -c 'import json,sys; sys.exit(0 if any(k["path"].startswith("accounts.'"$2"'.") for k in json.load(sys.stdin)["key"]) else 1)'
}
ai_has_account() { has_row_of ai_describe "$1"; }
sync_has_account() { has_row_of sync_describe "$1"; }
# app_title APP: what the person reads for the app: the `name` of its row in the system caller table ("Assistant" for
# org.bardo.Companion), else its id.
app_title() {
  python3 -I -c '
import sys, tomllib
with open(sys.argv[1], "rb") as f:
    rows = tomllib.load(f).get("caller", [])
print(next((r["name"] for r in rows if r.get("app") == sys.argv[2] and r.get("name")), sys.argv[2]))' "$ACC_SYSTEM_CALLERS" "$1"
}
# ai_grant_path APP: the path of APP's permit row on the AI keys (`accounts.<id>.grant.<grant>`), or nothing. The row's label
# starts with the app's title ("Assistant can use Language model", "org.bardo.Always can use Language model").
ai_grant_path() {
  local title; title=$(app_title "$1")
  ai_describe 2>/dev/null | python3 -I -c '
import json, re, sys
for k in json.load(sys.stdin)["key"]:
    p = k["path"]
    if ".grant." in p and not re.search(r"\.(since|scope|app|service)$", p) and str(k.get("label", "")).startswith(sys.argv[1] + " "):
        print(p); break' "$title"
}
# ai_grant_field APP scope|decision: the permit's `scope` (once, always, session), or its decision (allow, deny); `none` when APP has none.
ai_grant_field() {
  local path; path=$(ai_grant_path "$1")
  [ -n "$path" ] || { echo none; return; }
  case "$2" in
    scope) ai_get "$path.scope" ;;
    decision) ai_describe | python3 -I -c '
import json, sys
for k in json.load(sys.stdin)["key"]:
    if k["path"] == sys.argv[1]: print("deny" if " can'"'"'t " in str(k.get("label", "")) else "allow")' "$path" ;;
  esac
}

# --- the generic mail account of scenarios 1, 4 and 9 -------------------------------------------------------
# acc_fake_mail_provider: a brand file (discovery fixed) that names the rig's IMAP and SMTP servers. The generic
# provider (`generic-imap`, "Other...") cannot reach a fake in the jail: it finds servers by autoconfig over HTTPS
# on port 443 and, failing that, asks for a host and dials 993 and 587, ports the jail cannot bind.
# Needs the rig started with --imap --smtp --tls implicit.
acc_fake_mail_provider() {
  cat >"$PROV/fake-mail.toml" <<EOT
id = "fake-mail"
label = "Fake Mail"
mark = "mail"

[auth]
kind = "password"

[discovery]
kind = "fixed"

[[capability]]
family = "imap"
endpoint = "imaps://127.0.0.1:$(rigj imap.port)"
kind = "mail"
v = { access = "read_write", send = "present", delta = "poll", transport = "imap", labels = "folders" }

[[capability]]
family = "smtp"
endpoint = "smtps://127.0.0.1:$(rigj smtp.port)"
kind = "mail"
v = { access = "read_write", send = "present", delta = "poll", transport = "imap", labels = "folders" }
EOT
}

# acc_add_start APP-ID [PROVIDER]: AddAccount as that app (account-rig-client add-account), in the background; its answer
# lands in $OUT_DIR/add.log as one line: {"result":"added","account":..} or {"result":"refused",..}.
acc_add_start() {
  : >"$OUT_DIR/add.log"
  rigc_bg "$OUT_DIR/add.log" "$1" add-account ${2:+"$2"}
  ADD_PID=$ACC_LAST
}
# acc_add_response [SECONDS]: waits for the answer line and prints it.
acc_add_response() {
  acc_wait "${1:-30}" grep -q '"result"' "$OUT_DIR/add.log" || return 1
  grep '"result"' "$OUT_DIR/add.log" | tail -n 1
}
# acc_added RESPONSE: the account was added; sets ACCOUNT.
acc_added() {
  ACCOUNT=$(python3 -I -c '
import json, sys
m = json.loads(sys.argv[1])
if m.get("result") != "added": sys.exit(1)
print(m["account"])' "$1") || return 1
}
HARNESS_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# --- the secret scan -----------------------------------------------------------------------------------------
# scan_begin: records the private bus from now on (call it before the daemons start, so the names' owners are seen).
# scan_end: stops the record and runs harness/lib/secret-scan.py over it, the scenario's logs and the state, config
# and data trees of the scratch HOME for every secret the rig planted and the issuer's minted tokens. 0 clean.
scan_begin() { monitor_start; sleep 0.5; }
scan_end() {
  local args=() s files=()
  sleep 0.5; monitor_stop
  # What bardo-accountd kept (the registry and the audit file), for whoever reads the run afterwards.
  rm -rf "$OUT_DIR/state-after"; cp -r "$STATE" "$OUT_DIR/state-after" 2>/dev/null
  while read -r s; do
    [ -n "$s" ] || continue
    case "$s" in *ACCESS-TOKEN*) args+=(--access-secret "$s") ;; *) args+=(--secret "$s") ;; esac
  done < <(rigj secrets 2>/dev/null; [ ! -f "$RIG_DIR/sync/rig.json" ] || frigj secrets 2>/dev/null)
  local f
  for f in "$OUT_DIR"/*.log; do
    case "$(basename "$f")" in keys.log|input.log|rig-servers.log|sync-rig-servers.log) ;; *) files+=(--file "$f") ;; esac
  done
  python3 -I "$HARNESS_LIB/secret-scan.py" --bus "$OUT_DIR/bus.jsonl" "${args[@]}" \
    --pattern 'fake-refresh-[0-9]+' --pattern 'fake-code-[0-9]+' --access-pattern 'fake-access-[0-9]+' \
    "${files[@]}" --dir "$STATE" --dir "$CONFIG" --dir "$DATA" --report "$OUT_DIR/secret-scan.txt"
}

# sheet_add_fake_mail [SHOT-PREFIX]: the whole add flow of the fake mail account through the sheet as an app asks
# for it (the steps accounts-generic-mail checks one by one); the account is `fake-mail-ada-rig.test`.
sheet_add_fake_mail() {
  acc_add_start org.bardo.Mail
  sheet_await || return 1; sheet_settled; sleep 1
  sheet_type Fake; sleep 0.5; sheet_key Enter; sleep 1
  sheet_type "$(rigj imap.user)"; sheet_key Tab; sheet_type "$(rigj imap.password)"; sleep 0.5
  sheet_key Enter; sleep 2
  sheet_key Enter
  local response
  response=$(acc_add_response 30) || { echo "AddAccount never answered: $(cat "$OUT_DIR/add.log")" >&2; return 1; }
  acc_added "$response" || { echo "AddAccount answered $response" >&2; return 1; }
  sheet_gone
}

# sheet_add_microsoft: the whole browser sign-in through the sheet as an app asks for it (accounts-oauth-browser checks
# the steps one by one); needs the sync service's rig started (sync_rig_start), acc_clients_for_issuer and
# acc_provider_microsoft_on_rig. Sets ACCOUNT to the account's id.
sheet_add_microsoft() {
  : >"$RIG_DIR/opened.log"
  acc_add_start org.bardo.Mail
  sheet_await || return 1; sheet_settled; sleep 1
  sheet_type Micro; sleep 0.5; sheet_key Enter
  acc_wait 20 test -s "$RIG_DIR/opened.log" || { echo "bardo-shell never opened the browser: $(cat "$OUT_DIR/add.log")" >&2; return 1; }
  sleep 3; sheet_key Enter
  local response
  response=$(acc_add_response 30) || { echo "AddAccount never answered: $(cat "$OUT_DIR/add.log")" >&2; return 1; }
  acc_added "$response" || { echo "AddAccount answered $response" >&2; return 1; }
  sheet_gone
}

# sheet_add_openai: the whole add flow of an OpenAI key through the sheet (an AI key is added like any account: bardo-accountd
# draws the add sheet for Manager.AddAccount, the key goes sheet -> bardo-accountd -> Secret Service and the rig's LLM API fake
# checks it with its models list). Needs the rig started with --llm-api, acc_provider_openai_on_rig, bardo-accountd and bardo-shell.
# Sets ACCOUNT to the new account.
sheet_add_openai() {
  acc_add_start org.bardo.Companion
  sheet_await || return 1; sheet_settled; sleep 1
  sheet_type OpenAI; sleep 0.5; sheet_key Enter; sleep 1
  sheet_type "$(rigj llm_api.api_key)"; sleep 0.5
  sheet_key Enter; sleep 3
  sheet_key Enter
  local response
  response=$(acc_add_response 30) || { echo "AddAccount never answered: $(cat "$OUT_DIR/add.log")" >&2; return 1; }
  acc_added "$response" || { echo "AddAccount answered $response" >&2; return 1; }
  sheet_gone
}

# sheet_enter_until SECONDS CMD...: presses Return on whatever the sheet shows every 2 s until CMD succeeds (a sign-in's
# last cards: a review, a "Done"); fails when CMD still has not by then.
sheet_enter_until() {
  local until=$(( SECONDS + $1 )); shift
  while (( SECONDS < until )); do
    "$@" >/dev/null 2>&1 && return 0
    sheet_shown && sheet_key Enter
    sleep 2
  done
  "$@" >/dev/null 2>&1
}

# sync_files_turn_on ACCOUNT: Settings turns bardo-syncd's "Keep this account's files on this computer" on for an account bardo-accountd holds
# (Set accounts.<id>.sync.files s "on"), in the background: the call returns only after the person answers, because bardo-syncd's
# Keep::set_sync awaits Grants1.RequestGrant. bardo-syncd has no accounts of its own: when bardo-accountd holds no allowing grant of bardo-syncd's for
# the account, bardo-syncd asks (ConsentOffer.account is the account) and bardo-accountd draws its consent sheet for that one account (no
# chooser, no sign-in); one already held is not asked again. The answer is sync_consent_keys.
sync_files_turn_on() {
  { as_app org.bardo.Settings busctl --address="$BUS" --json=short call org.bardo.Sync1 $SYNC_SET org.bardo.SettingsModule1 Set sv \
      "accounts.$1.sync.files" s on >"$OUT_DIR/switch-on.log" 2>&1; } &
  ACC_PIDS+=("$!")
}
# sync_consent_keys: bardo-accountd's consent sheet for bardo-syncd's grant is up: Tab moves to Always Allow (Allow Once is first) and Return
# presses it, so the answer is kept as an `always` permit. Return alone is Allow Once, a permit bardo-accountd spends on its first use
# (account-service-grants relay.rs open_authenticated -> spend_once): bardo-syncd's first relay to Graph would take it back and the mirror
# would stop at the supervisor's next look.
sync_consent_keys() {
  sheet_settled; sleep 1
  sheet_key Tab; sheet_key Enter
}

# acc_permit_ids ACCOUNT APP: the ids of the permits (`accounts.<id>.grant.<grant>`) APP holds on ACCOUNT, as bardo-accountd's
# settings module lists them (the `.app` row of each permit is the app's name). bardo-accountd keeps every grant of an account
# there, AI or not; the apps using an identity (`used_by`) are Identities1.RegisterUse's list and name nobody bardo-syncd asked for.
acc_permit_ids() {
  local gid
  settings_describe 2>/dev/null | python3 -I -c '
import json, re, sys
pat = re.compile("^accounts\\." + re.escape(sys.argv[1]) + "\\.grant\\.(.+)\\.app$")
for k in json.load(sys.stdin)["key"]:
    m = pat.match(k["path"])
    if m: print(m.group(1))' "$1" | while read -r gid; do
    [ "$(settings_get "accounts.$1.grant.$gid.app" 2>/dev/null)" = "$2" ] && echo "$gid"
  done
}
# acc_permit_scope ACCOUNT APP: the `scope` (once, always, session) of APP's first permit on ACCOUNT, or `none`.
acc_permit_scope() {
  local gid; gid=$(acc_permit_ids "$1" "$2" | head -n 1)
  [ -n "$gid" ] || { echo none; return; }
  settings_get "accounts.$1.grant.$gid.scope"
}
SYNC_APP=org.bardo.Sync

# sheet_stray_escape: a sheet nobody expected is closed (so one failed check does not leave its sheet to take the next step's keys).
sheet_stray_escape() { sheet_shown && { sheet_key Escape; sheet_gone 8; }; }

# ai_request_start APP LOG: the app asks bardo-accountd for a permit to use the person's AI keys (account-rig-client ai-keys request),
# in the background; its one result line lands in LOG: {"result":"granted","permit":..} or {"result":"refused","refusal":..}.
ai_request_start() { rigc_bg "$OUT_DIR/$2.log" "$1" ai-keys request --class "${3:-files}"; }

# sheet_grant APP LOG: the app asks for a permit and the person answers Always Allow (Allow Once is first).
sheet_grant() {
  ai_request_start "$1" "$2"
  sheet_await || { cat "$OUT_DIR/$2.log" >&2; return 1; }
  sleep 1; sheet_key Tab; sheet_key Enter
  acc_wait 20 grep -q '"result"' "$OUT_DIR/$2.log" || { echo "ai-keys request never answered" >&2; return 1; }
  grep -q '"result":"granted"' "$OUT_DIR/$2.log" || { cat "$OUT_DIR/$2.log" >&2; return 1; }
  sheet_gone
}

# identity_of_account ID: the Identities1 id of an bardo-accountd account, as `identities list` prints it (the account's id when
# the list names none).
identity_of_account() {
  rigc "${2:-org.bardo.Mail}" identities list 2>/dev/null | python3 -I -c '
import json, sys
want = sys.argv[1]
for line in sys.stdin:
    try: m = json.loads(line)
    except ValueError: continue
    for i in m.get("identities", []):
        if str(i.get("id", "")) == want or want in json.dumps(i):
            print(i.get("id", want)); sys.exit(0)
print(want)' "$1"
}

# detent_start: bardo-settings (the Settings app, built at its pin) on bardo-compositor, on the private bus, named org.bardo.Settings
# (the role `settings` of the caller table). Its window is captured with `bardo-compositorctl shot --output` (acc_shot).
BARDO_SETTINGS_PID= PROBE_DIR=
detent_start() {
  local data="$SCRATCH/bardo-settings-data"
  # The programs' schemas, as bardo-settings's own tests seed them (its pages are made of schemas: without them the sidebar
  # lists only Accounts).
  mkdir -p "$data/bardo/settings" "$STATE/bardo/settings"
  cp "$SCEN_SIB"/settings-app/fixtures/schemas/*.toml "$data/bardo/settings/"
  # --first-run is skipped: the mark says it ran.
  echo ran >"$STATE/bardo/settings/first-run"
  # Not through `nested` (a function: the background job would be its subshell, not bardo-settings, and the identity written
  # below must be for the process that holds the bus connection).
  PROBE_DIR="$SCRATCH/probe"; mkdir -p "$PROBE_DIR"
  env BARDO_DEBUG_PROBE="$PROBE_DIR" WAYLAND_DISPLAY="$WL" XDG_RUNTIME_DIR="$(dirname "$WL")" HOME="$HOME_DIR" XDG_CONFIG_HOME="$CONFIG" XDG_DATA_HOME="$data" XDG_DATA_DIRS="$data" XDG_STATE_HOME="$STATE" \
    XDG_CACHE_HOME="$CACHE" DBUS_SESSION_BUS_ADDRESS="$BUS" "$SCEN_BUILD/detent-target/debug/bardo-settings" --dev-callers "$FLOOR_DIR/desktop-callers.toml" >"$OUT_DIR/bardo-settings.log" 2>&1 &
  BARDO_SETTINGS_PID=$!
  ACC_PIDS+=("$BARDO_SETTINGS_PID")
  acc_app org.bardo.Settings "$BARDO_SETTINGS_PID"
}

# acc_shot NAME: the whole output, as the compositor composes it (bardo-compositorctl shot --output).
acc_shot() { bardo-compositorctl shot --output HEADLESS-1 "$OUT_DIR/$1.png" >>"$OUT_DIR/shot.log" 2>&1; }

# detent_frame: "X Y W H" of bardo-settings's window in the output (bardo-compositorctl debug windows, the first window).
detent_frame() {
  bardo-compositorctl debug windows | python3 -I -c '
import json, sys
w = json.load(sys.stdin)
print(*[int(v) for v in w[0]["frame"]])'
}
# compositor_tap X Y: a left click at (X, Y) in the output (the pointer is moved first and given a moment: the app's hover
# state must catch up before a press counts).
compositor_tap() {
  bardo-compositorctl debug inject "move $1 $2" >/dev/null || return 1
  bardo-compositorctl debug wait settled --timeout 10000 >/dev/null 2>&1
  sleep 0.3
  bardo-compositorctl debug inject "press left" "release left" >/dev/null || return 1
  sleep 0.7
}

# detent_click RX RY: a left click at (RX, RY) relative to bardo-settings's window origin. A negative RX counts from the window's
# right edge. Only for what has no name (see detent_press for the rest).
detent_click() {
  local x y w h
  read -r x y w h < <(detent_frame) || return 1
  local rx=$1
  [ "$rx" -ge 0 ] || rx=$((w + rx))
  compositor_tap $((x + rx)) $((y + $2))
}

# --- bardo-settings's controls by role and name ----------------------------------------------------------------------------
# bardo-settings runs with BARDO_DEBUG_PROBE (detent_start), built with ui-blitz's `debug-probe` feature (scenario-build.sh):
# its window lists every control by ARIA role and accessible name with its box (Bardo's ui-blitz probe). A scenario
# presses "button Manage", never a place. The roles are the ones the markup has: button, switch, option (a sidebar row),
# checkbox, textbox ...; the name is the aria-label, else the visible text.

# detent_centre ROLE NAME [NTH]: "X Y", the control's centre in bardo-settings's window; exit 1 when there is none (bardo-settings not
# started yet: none). NTH counts from 1 in document order; -1 is the last (an alert's button, after the page's of the same
# name).
detent_centre() { [ -n "$BARDO_SETTINGS_PID" ] || return 1; python3 -I "$ACC_LIB/probe-find.py" "$PROBE_DIR" "$BARDO_SETTINGS_PID" "$1" "$2" ${3:+--nth "$3"}; }

# detent_state ROLE NAME [NTH]: the control's state words (checked, pressed, selected, expanded, disabled, or -).
detent_state() { [ -n "$BARDO_SETTINGS_PID" ] || return 1; python3 -I "$ACC_LIB/probe-find.py" "$PROBE_DIR" "$BARDO_SETTINGS_PID" "$1" "$2" ${3:+--nth "$3"} --state; }

# detent_listing: what bardo-settings shows now, one control per line (role x y w h state name).
detent_listing() { [ -z "$BARDO_SETTINGS_PID" ] || cat "$PROBE_DIR/$DETENT_PID-0.tsv" 2>/dev/null; }

# detent_press ROLE NAME [NTH]: waits for the control, waits until it stands still (two reads 0.3 s apart agree: a page
# change slides), scrolls it into the window when the probe lists it below the window's bottom edge (the sidebar is
# taller than the window, so its last pages are listed but out of reach), then clicks its centre as detent_click would.
detent_press() {
  local role=$1 name=$2 nth=${3:-} c1 c2 x y w h i cx cy
  for i in $(seq 100); do
    c1=$(detent_centre "$role" "$name" $nth) && { sleep 0.3; c2=$(detent_centre "$role" "$name" $nth) && [ "$c1" = "$c2" ] && break; }
    sleep 0.1
  done
  [ -n "${c2:-}" ] && [ "$c1" = "$c2" ] || {
    echo "bardo-settings shows no $role \"$name\" (it shows:)" >&2; detent_listing >&2; return 1
  }
  read -r x y w h < <(detent_frame) || return 1
  read -r cx cy <<<"$c1"
  # The wheel over the control's own column, a notch at a time, until its centre is inside the window; the page slides
  # while it scrolls, so the position is read again.
  for i in $(seq 20); do
    [ "$cy" -lt $((h - 16)) ] && break
    bardo-compositorctl debug inject "move $((x + cx)) $((y + h / 2))" "axis vertical 15" >/dev/null || return 1
    sleep 0.5
    c1=$(detent_centre "$role" "$name" $nth) || { echo "bardo-settings lost $role \"$name\" while scrolling to it" >&2; return 1; }
    read -r cx cy <<<"$c1"
  done
  [ "$cy" -lt $((h - 16)) ] || { echo "detent_press: $role \"$name\" stays below the window (y $cy of $h) after scrolling" >&2; return 1; }
  compositor_tap $((x + cx)) $((y + cy))
}

# detent_focus: the first press into a window only focuses it (bardo-settings's own grab zones take the next), so one harmless
# click on the title bar's text makes the following clicks count.
detent_focus() { detent_click 450 14; }

# sheet_settled: the sheet's drawing has stopped changing (two captures 0.6 s apart are the same): its first animation
# and the focus on its search field are done, so typed keys land.
sheet_settled() {
  local i
  for i in $(seq 30); do
    sheet_shot .settle-a; sleep 0.6; sheet_shot .settle-b
    [ "$(changed_fraction "$OUT_DIR/.settle-a.png" "$OUT_DIR/.settle-b.png")" = 0.0 ] && return 0
    [[ "$(changed_fraction "$OUT_DIR/.settle-a.png" "$OUT_DIR/.settle-b.png")" == 0.000* ]] && return 0
  done
  return 1
}

# acc_provider_openai_on_rig: the shipped openai.toml with its API at the rig's LLM API fake (needs --llm-api).
acc_provider_openai_on_rig() {
  sed -e "s#https://api.openai.com/v1#$(rigj llm_api.api_url)#" "$SCEN_SIB/accounts/providers/openai.toml" >"$PROV/openai.toml"
}

# sheet_click X Y: a click at surface coordinates (the sheet is drawn at the output's size) through `bardo-shell debug click`.
sheet_click() { bardo-shell debug click "$SHEET" "$1" "$2" >>"$OUT_DIR/keys.log" 2>&1; sleep 0.4; }
# sheet_replace X Y TEXT: click the text field at (X, Y), select its content and type TEXT.
sheet_replace() { sheet_click "$1" "$2"; sheet_key a+ctrl; sheet_type "$3"; sleep 0.3; }
