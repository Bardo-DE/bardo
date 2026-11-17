//! Starting every service: each on the runtime against `Env`'s seams, in the order
//! `shell_ui_kit::services` declares them, so a service's deps can only name the ones started
//! above it. A service whose system half is missing still yields its receiver, at its default.
//!
//! The kit declares what the services are (`Services`, its fields and fakes); this is the one place
//! that knows how they are wired to one another and to the daemon.

use std::fmt;

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
use shell_model::companion::{CompanionService, SummonOut};
use shell_model::desktop::{DesktopCommand, DesktopView};
use shell_model::displays::DisplaysService;
use shell_model::events::EventsService;
use shell_model::guest_asks::GuestAsksService;
use shell_model::idle::IdleService;
use shell_model::input_source::InputSourceService;
use shell_model::intents::IntentsService;
use shell_model::kbd_backlight::KbdBacklightService;
use shell_model::keycap::KeycapService;
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
use shell_model::session::SessionService;
use shell_model::sync_hold::SyncHoldService;
use shell_model::tray::TrayService;
use shell_model::voice::{VoiceConsent, VoiceService};
use shell_model::volume::VolumeService;
use shell_model::wallpaper::WallpaperService;
use shell_model::wifi::WifiService;
use shell_model::windows::WindowsService;
use shell_model::work_areas::WorkAreaService;
use shell_model::workspace_looks::WorkspaceLooksService;
use shell_search::results::{ResultsLinks, Sources};
use shell_services::accounts::sheet::AccountsSheetLinks;
use shell_services::capture_pick::CaptureLinks;
use shell_services::displays::bus::DisplaysBusLinks;
use shell_services::env::Env;
use shell_services::guest_asks::GuestAsksLinks;
use shell_services::launch::LaunchLinks;
use shell_services::seat::{SeatPublisher, SeatView, seat};
use shell_services::service::spawn;
use shell_services::sync_hold::SyncHoldLinks;
use shell_services::{
    AccountsLinks, Answers, AppMenuDeps, CompanionLinks, IdleLinks, IntentsLinks, KeycapLinks,
    MenuLinks, SessionLinks, ShellCall, VoiceLinks, WindowsLinks,
};
use shell_ui_kit::app_menu::menu_name_of;
use shell_ui_kit::dock_rects::DockRects;
use shell_ui_kit::services::Services;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch;

/// The ends the daemon holds, and the channels between services that no service owns.
pub struct Wiring {
    /// Where the shell's own actions go.
    pub shell_calls: UnboundedSender<ShellCall>,
    /// Where a summon's launcher effects go.
    pub summons: UnboundedSender<SummonOut>,
    /// Where the answer to the voice consent sheet goes.
    pub consents: UnboundedSender<VoiceConsent>,
    /// Where the answer to a question the compositor asked goes.
    pub permission_answers: UnboundedSender<DesktopCommand>,
    /// The connection the intents service claims `org.bardo.Shell` on, which the companion
    /// speaks on as the launcher.
    pub seat_publisher: SeatPublisher,
    pub seat_view: SeatView,
    /// The windows service writes the desktop's view; every reader of it holds the receiver.
    pub desktop_tx: watch::Sender<DesktopView>,
    pub desktop: watch::Receiver<DesktopView>,
    /// What the person does to a notification, which the services with notices hear.
    pub answers: Answers,
}

impl Wiring {
    /// The daemon's ends, with a seat nobody holds yet and an empty desktop.
    #[must_use]
    pub fn new(
        shell_calls: UnboundedSender<ShellCall>,
        summons: UnboundedSender<SummonOut>,
        consents: UnboundedSender<VoiceConsent>,
        permission_answers: UnboundedSender<DesktopCommand>,
    ) -> Wiring {
        let (seat_publisher, seat_view) = seat();
        let (desktop_tx, desktop) = watch::channel(DesktopView::default());
        Wiring {
            shell_calls,
            summons,
            consents,
            permission_answers,
            seat_publisher,
            seat_view,
            desktop_tx,
            desktop,
            answers: Answers::default(),
        }
    }
}

impl fmt::Debug for Wiring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wiring").finish_non_exhaustive()
    }
}

/// Start every service on `rt` against `env`'s seams, each after the ones it depends on.
///
/// The final struct literal names every field of `Services`: a service added to the kit's registry
/// and not started here does not compile.
#[must_use]
pub fn start(rt: &tokio::runtime::Handle, env: &Env, wiring: Wiring) -> Services {
    let apps = spawn::<AppsService>(rt, env, ());
    let notifications = spawn::<NotificationsService>(rt, env, wiring.answers.clone());
    let launch = spawn::<LaunchService>(
        rt,
        env,
        LaunchLinks {
            apps: apps.state.clone(),
            notifications: notifications.control.clone(),
        },
    );
    let companion = spawn::<CompanionService>(
        rt,
        env,
        CompanionLinks {
            seat: wiring.seat_view.clone(),
            summons: wiring.summons,
            launch: launch.control.clone(),
        },
    );
    let voice = spawn::<VoiceService>(
        rt,
        env,
        VoiceLinks {
            seat: wiring.seat_view.clone(),
            companion: companion.control.clone(),
            consents: wiring.consents,
        },
    );
    let volume = spawn::<VolumeService>(rt, env, ());
    let displays = spawn::<DisplaysService>(rt, env, volume.state.clone());
    let displays_bus = spawn::<DisplaysBusService>(
        rt,
        env,
        DisplaysBusLinks {
            displays: displays.state.clone(),
            control: displays.control.clone(),
        },
    );
    let capture = spawn::<CaptureService>(
        rt,
        env,
        CaptureLinks {
            seat: wiring.seat_view.clone(),
            displays: displays.state.clone(),
            desktop: wiring.desktop.clone(),
            apps: apps.state.clone(),
            answers: wiring.permission_answers,
        },
    );
    let battery = spawn::<BatteryService>(rt, env, ());
    let windows = spawn::<WindowsService>(
        rt,
        env,
        WindowsLinks {
            desktop: wiring.desktop_tx,
        },
    );
    let clipboard = spawn::<ClipboardService>(rt, env, ());
    let locker = spawn::<LockerService>(rt, env, ());
    let session = spawn::<SessionService>(
        rt,
        env,
        SessionLinks {
            locker: locker.control.clone(),
            windows: windows.state.clone(),
            wl: windows.control.clone(),
            apps: apps.state.clone(),
            notifications: notifications.control.clone(),
        },
    );
    let results = spawn::<ResultsService>(
        rt,
        env,
        Sources::Services(ResultsLinks {
            apps: apps.state.clone(),
            windows: windows.state.clone(),
            clipboard: clipboard.state.clone(),
            companion: companion.state.clone(),
            companion_control: companion.control.clone(),
            session: session.state.clone(),
            wl: windows.control.clone(),
        }),
    );
    let auth = spawn::<AuthService>(rt, env, ());
    let idle = spawn::<IdleService>(
        rt,
        env,
        IdleLinks {
            lock: locker.state.clone(),
            battery: battery.state.clone(),
            locker: locker.control.clone(),
            auth: auth.control.clone(),
        },
    );
    let screenshots = spawn::<ScreenshotsService>(rt, env, ());
    let keycap = spawn::<KeycapService>(
        rt,
        env,
        KeycapLinks {
            windows: windows.state.clone(),
            apps: apps.state.clone(),
            notifications: notifications.control.clone(),
            notes: notifications.state.clone(),
        },
    );
    let accounts = spawn::<AccountsService>(
        rt,
        env,
        AccountsLinks {
            notifications: notifications.control.clone(),
            notes: notifications.state.clone(),
            answers: wiring.answers.clone(),
        },
    );
    let accounts_sheet = spawn::<AccountsSheetService>(
        rt,
        env,
        AccountsSheetLinks {
            launch: launch.control.clone(),
        },
    );
    let sync_hold = spawn::<SyncHoldService>(
        rt,
        env,
        SyncHoldLinks {
            notifications: notifications.control.clone(),
            notes: notifications.state.clone(),
            answers: wiring.answers.clone(),
            launch: launch.control.clone(),
        },
    );
    let guest_asks = spawn::<GuestAsksService>(
        rt,
        env,
        GuestAsksLinks {
            notifications: notifications.control.clone(),
            notes: notifications.state.clone(),
            answers: wiring.answers.clone(),
        },
    );
    let night_shift = spawn::<NightShiftService>(rt, env, displays.state.clone());
    let clock = spawn::<ClockService>(rt, env, ());
    let network = spawn::<NetworkService>(rt, env, ());
    let wifi = spawn::<WifiService>(rt, env, ());
    let bluetooth = spawn::<BluetoothService>(rt, env, ());
    let kbd_backlight = spawn::<KbdBacklightService>(rt, env, ());
    let input_source = spawn::<InputSourceService>(rt, env, ());
    let power_profile = spawn::<PowerProfileService>(rt, env, ());
    let now_playing = spawn::<NowPlayingService>(rt, env, ());
    let badges = spawn::<BadgesService>(rt, env, ());
    let tray = spawn::<TrayService>(rt, env, ());
    let app_menu = spawn::<AppMenuService>(
        rt,
        env,
        AppMenuDeps {
            name_of: menu_name_of,
            windows: windows.state.clone(),
        },
    );
    let pins = spawn::<PinsService>(rt, env, apps.state.clone());
    let wallpaper = spawn::<WallpaperService>(rt, env, ());
    let events = spawn::<EventsService>(rt, env, ());
    let polkit_agent = spawn::<PolkitAgentService>(rt, env, ());
    let intents = spawn::<IntentsService>(
        rt,
        env,
        IntentsLinks {
            calls: wiring.shell_calls,
            menus: MenuLinks {
                toplevels: windows.state.clone(),
                wl: windows.control.clone(),
                name_of: menu_name_of,
            },
            shots: screenshots.state.clone(),
            seat: wiring.seat_publisher,
        },
    );
    let work_areas = spawn::<WorkAreaService>(rt, env, ());
    let workspace_looks = spawn::<WorkspaceLooksService>(rt, env, ());
    Services {
        apps,
        notifications,
        launch,
        companion,
        voice,
        volume,
        displays,
        displays_bus,
        capture,
        battery,
        windows,
        clipboard,
        locker,
        session,
        results,
        auth,
        idle,
        screenshots,
        keycap,
        accounts,
        accounts_sheet,
        sync_hold,
        guest_asks,
        night_shift,
        clock,
        network,
        wifi,
        bluetooth,
        kbd_backlight,
        input_source,
        power_profile,
        now_playing,
        badges,
        tray,
        app_menu,
        pins,
        wallpaper,
        events,
        polkit_agent,
        intents,
        work_areas,
        workspace_looks,
        desktop: wiring.desktop,
        dirs: env.dirs.clone(),
        dock_rects: DockRects::default(),
        session_reach: env.session_actions.reach(),
    }
}
