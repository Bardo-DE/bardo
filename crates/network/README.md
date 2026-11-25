# airwaves

Typed [zbus](https://crates.io/crates/zbus) proxies for the two system services a Linux desktop's
network and Bluetooth controls talk to:

- `networkmanager-proxy`: NetworkManager (`org.freedesktop.NetworkManager`): the manager, devices, Wi-Fi,
  access points, active and saved connections, and enums for the kinds and states it sends.
- `bluez-proxy`: BlueZ (`org.bluez`): adapters, devices, batteries, the agent manager, and the
  errors a pairing agent answers with.

Written from the services' documented D-Bus APIs. Nothing links libdbus, and nothing here is tied to
a desktop: any zbus program on any Linux distribution can use them.

## Use

Pin by tag:

```toml
networkmanager-proxy = { git = "https://github.com/Bardo-DE/bardo", tag = "v0.1.0", default-features = false, features = ["tokio"] }
```

zbus runs on its own executor by default (`async-io`); a tokio program turns default features off
and `tokio` on.

```rust,ignore
let bus = zbus::Connection::system().await?;
let nm = airwaves_nm::NetworkManagerProxy::new(&bus).await?;
for path in nm.get_all_devices().await? {
    let device = airwaves_nm::DeviceProxy::builder(&bus).path(path)?.build().await?;
    let kind = airwaves_nm::DeviceType::from_wire(device.device_type().await?);
}
```

## Licence

MIT or Apache-2.0, at your option.
