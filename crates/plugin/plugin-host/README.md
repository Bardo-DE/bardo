# The plugin host

The plugin host runs plugins as separate programs. A plugin is an executable a person installs; the host
never links it. The plugin describes itself in a TOML manifest, the host finds the manifest, starts
the program for one request, and talks to it over a pipe in length-prefixed JSON frames. A plugin
that crashes, hangs or lies costs one request and never takes the host down.

It is generic over the host's *capability type*, the things a plugin can offer (`thumbnail`,
`block.lens`, `open.provider`). The plugin host owns the mechanics and none of the vocabulary:

The work is split in three crates:

| Crate | What it does |
|---|---|
| `plugin-core` | The pure half, with no process and no file. `wire`: frames (`u32 LE json length`, `u32 LE payload length`, JSON, payload), the blocking `read_frame` and `write_frame` a plugin uses, and the incremental `FrameDecoder` a host uses. `manifest`: the envelope every manifest has (`id`, `name`, `protocol`, optional `version`, `licence`, `summary`, `[program]`), checked once; each `[[provides]]` table goes to the host's own `Provides` type, or is kept as `Raw` with its `capability` word for an owner to read later. `registry`: which plugin serves a request (newer protocol first, then the person's directory over the system's, then the newer `version`, then by id, so the order the disk listed files in never matters). `reparse_with`, `suggest`, `Timeouts`. A plugin program depends on this crate alone. |
| `plugin-host` (this crate) | `discover`: finds `<data dir>/<app>/plugins/*.toml` (the flat `Layout`) or `<data dir>/<app>/<dir>/<id>/<manifest name>` (`Layout::Folders`, the folder name is the id and paths stay inside the folder) under the person's directory and the system's, with the app name a parameter, and checks that the programs they name are there. `scan` stops before ranking; `reparse` turns a `Candidate<Raw>` into the owner's `Candidate<P>`, reporting entries of words the owner does not own as skipped. `run` (Unix): starts the program in a process group of its own, takes its greeting, sends the request, reads the answer with a deadline, supports cancellation with a grace period, and kills and reaps the whole group on every path out. |
| `plugin-fake` | A fake plugin for a host's tests (dev-only). |

## Using it

A host implements two small traits.

```rust,ignore
// One `[[provides]]` entry of a manifest, parsed into the host's own type.
impl plugin_core::manifest::Provides for Provision { /* Capability, Fault, parse, capability */ }

// The messages the host and its plugins exchange, and the greeting that starts them.
impl plugin_host::run::Protocol for Wire { /* Provides, Request, Message, VERSION, greeting */ }
```

Then:

```rust,ignore
let found = plugin_host::discover::<Provision>(&Search::new("myapp", user, system, Wire::VERSION));
let plugin = found.registry.serving(Cap::Thumbnail, |p| fit(p, &file)).ok_or(missing)?;
let mut session = Runner::new("myapp", Timeouts::default().with_silence(Duration::from_secs(10)))
    .open::<Wire>(plugin, Cap::Thumbnail, &request, payload_limit)?;
let reply = session.reply()?;           // one answer, or
session.stream(cancelled, &Request::Cancel, |id, frame| /* ... */)?;  // a stream of them
```

For a layout of one folder per plugin, with a vocabulary owned by several crates:

```rust,ignore
let search = Search::new("bardo", user, system, 1)
    .with_layout(Layout::Folders { dir: "extensions", manifest: "extension.toml" });
let scanned = plugin_host::scan::<Raw>(&search);            // knows no capability word
let typed = scanned.candidates.into_iter()
    .map(plugin_host::reparse::<Mine>)                       // the owner's words; others are skipped
    /* ...the owner's own checks... */;
let registry = plugin_core::registry::Registry::resolve(candidates, Wire::VERSION);
```

A plugin program (depending on `plugin-core` only) is a loop over `read_frame` and `write_frame` on its standard streams. It speaks
first, with its greeting. `examples/demo_plugin.rs` is the shortest complete one.

## Capabilities of other hosts

`docs/capabilities.md` shows how four capabilities of a document host (`block.lens`, `open.provider`,
`agent.adapter`, `row.facts`) sit on this API, and how `open.provider` has the wire shape of a
launcher's provider, so one plugin can feed a launcher and a document host. `tests/it/capabilities.rs` runs
them against a real plugin program.

## Fakes and portability

- `plugin_fake::Fake` (crate `plugin-fake`, a dev-dependency of a host): a fake plugin for a host's
  tests. It greets, hands each request to a closure of the host's, and misbehaves on the wire on
  request (`--fault mute`, `garbage`, `ignore-cancel`, and the rest of `Fault`).
  `examples/fake_plugin.rs` is one in a few lines.
- The wire, manifests and the registry (`plugin-core`) and discovery build on any platform. `run`
  needs Unix (process groups, `poll`) and is absent elsewhere. There is no bus, no UI toolkit and
  no async runtime in the tree; `cargo deny` bans them.

## Run, never link

The plugin host exists so that copyleft and codec code is never linked into an application.

## Licence

MIT OR Apache-2.0, at your option. See `LICENSE-MIT` and `LICENSE-APACHE`.
