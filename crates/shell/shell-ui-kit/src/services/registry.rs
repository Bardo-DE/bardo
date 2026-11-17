//! Every service, declared once, in the order they start: what it is and (behind `harness`) the
//! state its fake starts at when that is not its default. To add a service, add its line here and
//! its spawn in `shell-app`'s `start` (after the services in its deps); the [`Services`] field,
//! its [`Member`](super::Member) impl and the fake follow.

use shell_model::accounts::AccountsService;
use shell_model::accounts::sheet::AccountsSheetService;
use shell_model::app_menu::AppMenuService;
use shell_model::apps::AppsService;
use shell_model::auth::AuthService;
use shell_model::badges::BadgesService;
use shell_model::battery::BatteryService;
use shell_model::bluetooth::BluetoothService;
use shell_model::capture_pick::CaptureService;
use shell_model::clipboard::ClipboardService;
use shell_model::clock::ClockService;
use shell_model::companion::CompanionService;
use shell_model::desktop::DesktopView;
use shell_model::dirs::Dirs;
use shell_model::displays::DisplaysService;
use shell_model::events::EventsService;
use shell_model::guest_asks::GuestAsksService;
use shell_model::idle::IdleService;
use shell_model::input_source::InputSourceService;
use shell_model::intents::IntentsService;
use shell_model::kbd_backlight::KbdBacklightService;
use shell_model::key_remap::KeyRemapService;
use shell_model::launch::LaunchService;
use shell_model::locker::LockerService;
use shell_model::network::NetworkService;
use shell_model::night_shift::NightShiftService;
use shell_model::notifications::NotificationsService;
use shell_model::now_playing::NowPlayingService;
use shell_model::pins::PinsService;
use shell_model::polkit_agent::PolkitAgentService;
use shell_model::power_profile::PowerProfileService;
use shell_model::results::ResultsService;
use shell_model::screenshots::ScreenshotsService;
use shell_model::serve::DisplaysBusService;
use shell_model::session::{ActionReach, SessionService};
use shell_model::sync_hold::SyncHoldService;
use shell_model::tray::TrayService;
use shell_model::voice::VoiceService;
use shell_model::volume::VolumeService;
use shell_model::wallpaper::WallpaperService;
use shell_model::wifi::WifiService;
use shell_model::windows::WindowsService;
use shell_model::work_areas::WorkAreaService;
use shell_model::workspace_looks::WorkspaceLooksService;
use tokio::sync::watch;

use crate::dock_rects::DockRects;

services! {
    {
        apps: AppsService;
        /// The notifications server's live banners and name holder, and its history. Passive until
        /// the daemon's policy.
        notifications: NotificationsService;
        launch: LaunchService;
        /// The companion: where a summon goes, its answers, the computer-use runs, the undo journal
        /// and the launcher's two lookups; it speaks as the shell on the connection `intents` holds.
        companion: CompanionService;
        /// Hold-to-talk: where the hold stands, the words heard so far and the consent sheet; it
        /// speaks to voiced as the shell, and asks the companion what it heard.
        voice: VoiceService;
        volume: VolumeService => crate::harness::fake::volume();
        /// Inert until the daemon's first policy.
        displays: DisplaysService;
        /// Each display's brightness for Settings (`org.bardo.Displays1`); passive until the daemon
        /// says to serve.
        displays_bus: DisplaysBusService;
        /// The screen-sharing picker the portal asks (`org.bardo.Shell1.PickCaptureSource`); served on
        /// the connection `intents` holds, passive until the daemon says to serve.
        capture: CaptureService;
        battery: BatteryService => crate::harness::fake::battery();
        windows: WindowsService => shell_model::windows::WindowModel {
            workspaces: crate::harness::fake::workspaces(),
            toplevels: crate::harness::fake::toplevels(),
            ..shell_model::windows::WindowModel::default()
        };
        /// The clipboard history: passive until the daemon's policy.
        clipboard: ClipboardService;
        /// bardo-shell's own lock screen; the session's Lock goes through it.
        locker: LockerService;
        session: SessionService;
        results: ResultsService;
        /// The lock screen's password check (idle until a secret is submitted).
        auth: AuthService;
        /// bardo-shell's own idle: passive until the daemon's policy, and `Off` anywhere but a bardo-shell
        /// session.
        idle: IdleService;
        /// Screenshots for the floating thumbnail; passive until the daemon configures it.
        screenshots: ScreenshotsService;
        /// The key remap daemon keyboard daemon: its connection, and the notices about it.
        key_remap: KeyRemapService;
        /// The accounts that need signing in again, and the notices that say so.
        accounts: AccountsService;
        /// The accounts sheet accountd opens on `org.bardo.AccountsSheet1`; passive until the daemon
        /// says to serve.
        accounts_sheet: AccountsSheetService;
        /// Datasets whose removal syncd holds back, and the notices that say so.
        sync_hold: SyncHoldService;
        /// Computers of the person's asking to borrow this one's models, and the notices that ask.
        guest_asks: GuestAsksService;
        /// The compositor's night light, asked for by the person (the control center's Night Shift); it
        /// reads and writes the compositor's settings file, and tells the compositor which monitors the
        /// displays service warms itself.
        night_shift: NightShiftService;
        clock: ClockService => crate::harness::fake::clock();
        network: NetworkService => crate::harness::fake::network();
        wifi: WifiService => crate::harness::fake::wifi();
        bluetooth: BluetoothService => crate::harness::fake::bluetooth();
        /// The keyboard backlight (design/26 5.2.7): absent on a machine without one.
        kbd_backlight: KbdBacklightService;
        /// Next Input Source: switches the running input-method framework (fcitx5) to its next
        /// input method; it reaches the session bus only when asked.
        input_source: InputSourceService;
        power_profile: PowerProfileService => crate::harness::fake::power_profiles();
        now_playing: NowPlayingService;
        badges: BadgesService;
        tray: TrayService;
        /// The menus the focused apps serve (`com.canonical.dbusmenu`): empty until a bar names an app
        /// that has one.
        app_menu: AppMenuService;
        pins: PinsService;
        wallpaper: WallpaperService;
        /// Calendar events for the Calendar and Up Next widgets; passive until the daemon
        /// configures it.
        events: EventsService;
        /// The polkit agent (passive until the daemon's policy).
        polkit_agent: PolkitAgentService;
        /// bardo-shell's seat at intentd: the confirmation sheet it serves, the terminal's grants, the shell's
        /// own actions as `org.bardo.Shell`; passive until the daemon says to serve.
        intents: IntentsService;
        /// Each output's work area and scale, served on `org.bardo.Outputs1`; passive until the daemon
        /// says to serve.
        work_areas: WorkAreaService;
        /// Each workspace's name, output and `SpaceLook`, served on `org.bardo.Workspaces1`; passive
        /// until the daemon says to serve.
        workspace_looks: WorkspaceLooksService;
    }
    {
        /// The desktop as the compositor's own protocol describes it (apps, windows, Spaces),
        /// projected by the windows service from the state `windows` is projected from. The one
        /// channel every reader of the desktop's view holds; empty on any other compositor.
        desktop: watch::Receiver<DesktopView>
            => watch::Sender::new(DesktopView::default()).subscribe();
        /// The person's directories the services were started with (the icon theme's config, the
        /// widget placements' state folder).
        dirs: Dirs
            => Dirs::under(std::path::Path::new("/nonexistent/shell-ui-kit-test"));
        /// Where the docks' icons rest, for the compositor's minimise targets.
        dock_rects: DockRects => DockRects::default();
        /// Where the session service sends chosen actions (logind, or a dry run's record), for the
        /// surfaces' log lines.
        session_reach: ActionReach => ActionReach::DryRun;
    }
}
