//! `bardo-shell debug stats`: every surface's frame statistics, through the always-public
//! `HostCtx::take_stats` (it needs neither the `debug` feature nor layer-shell-host's `debug-inject`;
//! draining, so each report covers only the time since the previous one).

use layer_shell_host::HostCtx;
use layer_shell_types::SurfaceId;
use shell_ipc::debug::{Micros, SurfaceName, SurfaceStats};
use shell_overlays::LOCK;
use shell_settings::ShellFile;

use crate::daemon::msg::ShellMsg;
use crate::outputs::{OutputSurfaces, Outputs};

/// Every known surface's frame statistics: the rows' surfaces, and the lock surfaces while a lock
/// is up.
pub fn collect(ctx: &mut HostCtx<ShellMsg>, outputs: &Outputs) -> Vec<SurfaceStats> {
    let ids: Vec<(String, SurfaceId)> = outputs
        .outputs
        .iter()
        .flat_map(|(_, surfaces)| row_surfaces(surfaces))
        .collect();
    let locks = lock_surfaces(ctx, outputs);
    ids.into_iter()
        .chain(locks)
        .filter_map(|(name, id)| Some(wire_stats(name, &ctx.take_stats(id)?)))
        .collect()
}

/// The lock surfaces are the host's (layer-shell-host names each `shell-lock/<output>`), not a row of
/// `outputs`, so they are found by name.
fn lock_surfaces(ctx: &mut HostCtx<ShellMsg>, outputs: &Outputs) -> Vec<(String, SurfaceId)> {
    outputs
        .outputs
        .iter()
        .filter_map(|(info, _)| info.name.clone())
        .filter_map(|name| {
            let namespace = (LOCK.config)(&ShellFile::default()).namespace;
            let label = format!("{namespace}/{name}");
            let id = ctx.surface_named(&layer_shell_types::SurfaceName(label.clone()))?;
            Some((label, id))
        })
        .collect()
}

/// One output's spawned surfaces, each with its layer namespace.
fn row_surfaces(surfaces: &OutputSurfaces) -> Vec<(String, SurfaceId)> {
    surfaces
        .spawned
        .iter()
        .map(|surface| (surface.namespace.clone(), surface.id))
        .collect()
}

fn wire_stats(name: String, stats: &layer_shell_host::FrameStats) -> SurfaceStats {
    SurfaceStats {
        surface: SurfaceName(name),
        frames: stats.counters.frames,
        callbacks: stats.counters.callbacks,
        commits: stats.counters.commits,
        skipped: stats.counters.skipped,
        watchdog_fired: stats.counters.watchdog_fired,
        work: wire_micros(stats.work),
        interval: wire_micros(stats.interval),
        samples: stats.samples,
        vm_rss_kib: stats.vm_rss_kib,
    }
}

fn wire_micros(p: layer_shell_host::Percentiles) -> Micros {
    Micros {
        p50_us: p.p50_us,
        p95_us: p.p95_us,
        max_us: p.max_us,
    }
}
