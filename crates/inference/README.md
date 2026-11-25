# The inference service

The inference service is the AI broker of the desktop. One background service, `inferd`, lets apps use a model on
this computer, on another of your computers (if you allow it) or one you have an account for, under
the limits you set (what may leave the machine, how much it may spend). Apps ask for what they need
("a chat model that reads images"), never for a brand, and never see a key.

## What is here

| Crate | What it is |
| --- | --- |
| `inferd` | the daemon: wiring, the `org.bardo.Inference1` bus service and supervision, over the libraries below |
| `inference-client` | the app's client of inferd, over D-Bus or hosted in the app |
| `inference-dbus` | `org.bardo.Inference1` written once: proxies, server traits, introspection, bus vocabulary |
| `inference-rig` | test tooling, never installed: `inference-rig-client open`, an app opening an inferd session in a jailed scenario |
| `inference-probe` | the port probe that finds local runtimes (Ollama, llama.cpp, LM Studio) |
| `inference-core` | typed requests and replies, the routing policy, spend caps, audit entries |
| `inference-router` | who answers a session, the catalogue, the local model book, the session machine |
| `inference-fs` | the file io the pure router and bridge are given as values: the catalogue directories, a key file, a stale engine socket, a frame's descriptors |
| `inference-settings` | the `[ai]` rows as values: what they resolve to, their slugs and paths, the structured-reply limits and the spend book |
| `inference-agent-proto` | an external coding agent's Anthropic and OpenAI protocols as bytes in and out |
| `inference-turns` | running model turns: chat, embeddings, computer-use steps, structured replies |
| `inference-bridge` | the one mapping between the wire types and the model layer's provider types |
| `inference-tailnet` | lending models to the person's other computers over their Tailscale network |

`ARCHITECTURE.md` has the map and the allowed edges. The inference service depends on the accounts service by git rev; the accounts service
never depends on the inference service.

## Install

You need a Linux desktop with systemd and a Rust toolchain to build.

    scripts/install.sh                      # builds, then installs under /usr (run as root)
    scripts/install.sh --prefix /usr/local
    scripts/install.sh --destdir /tmp/stage # copies into a staging folder and touches nothing else

The script also reads `PREFIX` and `DESTDIR` from the environment. It can be run again at any time:
a file that is already right is left alone, and a caller table you edited is kept (the shipped one
is put beside it as `inferd-callers.toml.dist`). It never writes into a home folder. `--bin-dir DIR`
installs a daemon you built yourself instead of building it, and `--help` lists the rest.

Building needs access to the private repositories the dependencies come from (the accounts service, the model layer and
Bardo, as git dependencies); there is no public mirror yet.

## Where files go

Installed by the script (under the prefix, `/usr` by default):

| What | Where |
| --- | --- |
| The daemon | `libexec/bardo/bardo-inferd` |
| Its systemd user unit | `lib/systemd/user/bardo-inferd.service` |
| What lets the session bus start it | `share/dbus-1/services/org.bardo.Inference1.service` |
| The AI page's settings description | `share/bardo/settings/inference.settings.toml` |
| The file that lets inferd answer your other computers (kept as data; the Settings switch installs it, with your say-so) | `share/bardo/inference/tailnet.conf` |
| Examples to copy: the configuration and the optional file that lets it reach hosted models | `share/doc/bardo/examples/` |
| The table that says which app is Settings, which are the services | `/etc/bardo/inferd-callers.toml` |

## Checks

    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo deny check licenses
    scripts/check-boundary.sh
    scripts/check-features.sh

## Licence

MIT OR Apache-2.0.
