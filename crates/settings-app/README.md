# The Settings app

The desktop's Settings app (`org.bardo.Settings`): one window on Blitz, drawn with Bardo's
components. Its rows come from each app's settings schema, and its Accounts and Intelligence
pages talk to the account service and the inference service.

## Install

`scripts/install.sh` puts in place the binary, the desktop entry, the intents manifest, the
D-Bus service file and the systemd user unit `org.bardo.Settings.service`.

## Running as Settings

The account service gives the Settings role (reading and changing accounts, sign-in keys and sync
decisions) only to the main process of the user unit `org.bardo.Settings.service`. D-Bus
activation starts that unit, so open Settings from the desktop or through an intent. A Settings app
started any other way (from a terminal, from an app's scope, from another unit) is an ordinary app:
it gets no Settings role, and its Accounts page says it may not be shown the accounts.

## D-Bus surface

On the session bus the Settings app owns `org.bardo.Settings` and serves two interfaces:

- `org.bardo.IntentProvider1` at `/org/bardo/IntentProvider1` (the desktop's intents).
- `org.freedesktop.Application` at `/org/bardo/Settings` (D-Bus activation; the desktop entry
  says `DBusActivatable=true`):
  - `Activate(platform_data)` shows and raises the window (`activation-token` is honoured).
  - `Open(uris, platform_data)` answers NotSupported: Settings opens no files.
  - `ActivateAction("open-account", [account object path], platform_data)` shows the Accounts
    page at that account (`/org/bardo/Accounts1/account/<segment>`); an unknown account shows the
    list, and an unknown action is logged and ignored.

A second `bardo-settings` finds the name taken, calls `Activate` on the running one and exits.
