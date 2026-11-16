# bardo-portal

The desktop portal backend for the Bardo desktop: screenshots, screen sharing (PipeWire, outputs
and single windows, restore tokens), remote desktop, settings and the "keep the screen on" inhibit,
served to apps through `xdg-desktop-portal` as the `bardo` portal implementation.

It runs as its own process and talks to the compositor (casement) through the standard Wayland
image-capture protocols, so it works with any compositor that offers them. The file chooser stays
with `xdg-desktop-portal-gtk`.

Status: being built (phase P2 of casement). Licensed under either of MIT or Apache-2.0, at your
option.

## Debug builds

`cargo build -p portal-daemon --features debug` adds one thing: `BARDO_PORTAL_DEV_CALLERS=FILE`, a rows file of your own (peephole's
`[[peer]]` rows, a plain file that group and others cannot write) that names the stand-ins for the portal frontend and
the shell, for a jail or a checkout that has no `/etc/bardo/desktop-callers.toml`. Each row has `app`, `role`
(`portal_frontend` or `shell`) and `exe`. A release build has neither the feature nor the variable. See
`ARCHITECTURE.md` section 3b-2.

## Installing

`scripts/install.sh [--prefix DIR] [--destdir DIR] [--bin-dir DIR] [--no-build]` builds bardo-portal and installs the
binary (`libexec/bardo-portal`), `bardo-portal.service`, the D-Bus activation file and `bardo.portal`. The portal choice
(`dist/bardo-portals.conf`) is an example; the desktop ships the one it uses. The session must set
`XDG_CURRENT_DESKTOP=bardo` so the portal file's `UseIn=bardo` matches. The first portal call starts bardo-portal by
D-Bus activation; nothing is enabled under the home directory.
