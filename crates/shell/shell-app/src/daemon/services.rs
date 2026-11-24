//! Telling the services that are passive until told what to do, once the settings are read.
//! Starting them is `daemon::start`; what they are is declared in the kit's registry.

use shell_model::capture_pick::CaptureCommand;
use shell_model::locker::LockerCommand;
use shell_model::polkit_agent::PolkitCommand;
use shell_services::env::Env;
use shell_ui_kit::links::Links;
use shell_ui_kit::services::Services;
use shell_ui_kit::settings::SettingsFeed;
use tokio::runtime::Handle as Runtime;

use crate::policy::{
    calendar::CalendarBridge, clipboard::ClipboardBridge, companion::CompanionBridge,
    displays::DisplaysBridge, follow, idle::IdleBridge, keyboard::KeyboardBridge,
    night_shift::NightShiftBridge, notifications::NotificationsBridge,
    screenshots::ScreenshotsBridge, search::SearchBridge, voice::VoiceBridge,
    wallpaper::WallpaperBridge,
};

/// Tell each service that is passive until told what it should do: bardo-shell's own lock and polkit
/// agent, and the policies the settings decide (the first one sent ends a service's passive
/// start, and each is sent again whenever its answer changes).
pub(crate) fn follow_settings(
    rt: &Runtime,
    feed: &SettingsFeed,
    links: &Links,
    services: &Services,
    env: &Env,
) {
    let settings = || feed.settings.clone();
    // The notifications service's policy from the settings and the focus mode.
    follow(
        rt,
        settings(),
        NotificationsBridge {
            focus: links.focus.mode(),
        },
        services.notifications.control.clone(),
    );
    // The lock and the polkit agent are bardo-shell's own; the seat at intentd (the confirmation sheet,
    // the terminal's grants, `org.bardo.Shell`), the accounts sheet and the outputs' and
    // workspaces' names claim themselves at start (`Env::claims`).
    let _ = services.locker.control.send(LockerCommand::Configure);
    // The screen-sharing picker answers on `org.bardo.Shell1`, on the same connection.
    let _ = services.capture.control.send(CaptureCommand::Serve);
    let _ = services
        .polkit_agent
        .control
        .send(PolkitCommand::Policy(true));
    // bardo-shell's own idle follows `idle.*` and `session.lock_grace_s`, and is `Off` anywhere but a
    // bardo-shell session.
    let idle = IdleBridge {
        times: env.idle_times,
    };
    follow(rt, settings(), idle, services.idle.control.clone());
    // The launcher's providers and the clipboard history follow `launcher.*`.
    follow(
        rt,
        settings(),
        SearchBridge,
        services.results.control.clone(),
    );
    follow(
        rt,
        settings(),
        ClipboardBridge,
        services.clipboard.control.clone(),
    );
    // The displays follow `display.*`.
    follow(
        rt,
        settings(),
        DisplaysBridge,
        services.displays.control.clone(),
    );
    // Night Shift tells the compositor to leave the monitors warmed over DDC/CI alone.
    follow(
        rt,
        settings(),
        NightShiftBridge,
        services.night_shift.control.clone(),
    );
    // The companion's double tap and summon follow `companion.*`.
    follow(rt, settings(), VoiceBridge, services.voice.control.clone());
    follow(
        rt,
        settings(),
        CompanionBridge,
        services.companion.control.clone(),
    );
    // The key remap daemon's per-app profiles follow `keyboard.overrides`.
    follow(
        rt,
        settings(),
        KeyboardBridge,
        services.key_remap.control.clone(),
    );
    // The wallpaper follows `wallpaper.picture`.
    follow(
        rt,
        settings(),
        WallpaperBridge,
        services.wallpaper.control.clone(),
    );
    // The calendar's sources and the screenshots' folders and tool.
    let home = env.dirs.home.clone();
    let calendar = CalendarBridge { home: home.clone() };
    follow(rt, settings(), calendar, services.events.control.clone());
    let screenshots = ScreenshotsBridge { home };
    follow(
        rt,
        settings(),
        screenshots,
        services.screenshots.control.clone(),
    );
}

#[cfg(test)]
mod tests {
    use crate::daemon::start::{Wiring, start};
    use base_testkit::Scratch;
    use shell_services::env::Env;
    use shell_ui_kit::services::Services;

    /// `start` must never panic — the daemon has to reach the host connect and the
    /// IPC server — and every passive service holds its default (a service without its bus or
    /// compositor degrades to its own default, per `ServiceError`'s doc); the clock reads the
    /// time at once (`Thu 24 Sep 14:05`: four words).
    #[test]
    fn start_does_not_panic_and_passive_services_hold_their_defaults() {
        let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
        // No bus, no compositor, every directory in a scratch folder: nothing here reaches the
        // person's session or files.
        let scratch = Scratch::new("services-spawn");
        let (calls, _shell_calls) = tokio::sync::mpsc::unbounded_channel();
        let (summons, _launcher_effects) = tokio::sync::mpsc::unbounded_channel();
        let (consents, _written) = tokio::sync::mpsc::unbounded_channel();
        let services = start(
            rt.handle(),
            &Env::isolated(scratch.path()),
            Wiring::new(
                calls,
                summons,
                consents,
                tokio::sync::mpsc::unbounded_channel().0,
            ),
        );

        assert_eq!(
            services.held(),
            Services::DECLARED,
            "every declared service is started"
        );
        assert!(services.clock.state.borrow().0.year() > 2000);
        assert_eq!(
            services.notifications.state.borrow().live.server.mode,
            shell_model::notifications::ServerMode::Passive,
            "passive until the daemon's policy"
        );
        assert_eq!(
            services.events.state.borrow().status,
            shell_model::events::SourceStatus::Off,
            "no calendar read until the daemon names the sources"
        );
        assert_eq!(
            services.polkit_agent.state.borrow().registration,
            shell_model::polkit_agent::Registration::Passive,
            "no agent registered until the daemon's policy"
        );
        assert_eq!(
            services.auth.state.borrow().phase,
            shell_model::auth::AuthPhase::Idle,
            "nothing checked until a secret is submitted"
        );
        assert_eq!(
            services.clipboard.state.borrow().status.mode,
            shell_model::clipboard::HistoryMode::Off,
            "no clipboard read until the daemon's policy"
        );
        assert_eq!(
            *services.key_remap.state.borrow(),
            shell_model::key_remap::KeyRemapState::Unknown,
            "no socket and no system bus in an isolated Env: nothing is asked"
        );
        assert_eq!(
            services.intents.state.borrow().link,
            shell_model::intents::IntentsLink::Passive,
            "no bus name claimed until the daemon says to serve"
        );
        assert_eq!(
            services.accounts_sheet.state.borrow().link,
            shell_model::accounts::sheet::SheetLink::Passive,
            "no bus name claimed until the daemon says to serve"
        );
        assert_eq!(
            services.work_areas.state.borrow().link,
            shell_model::work_areas::WorkAreaLink::Passive,
            "no bus name claimed until the daemon says to serve"
        );
        assert_eq!(
            services.workspace_looks.state.borrow().link,
            shell_model::workspace_looks::WorkspaceLooksLink::Passive,
            "no bus name claimed until the daemon says to serve"
        );
        assert_eq!(
            services.idle.state.borrow().mode,
            shell_model::idle::IdleMode::Off(shell_model::idle::OffReason::NoPolicy),
            "no idle notification, display or logind touched until the daemon's policy"
        );
    }
}
