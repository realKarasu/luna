//! Luna: per-screen display tuning, a crosshair overlay and, on Windows, gaming tweaks.
//!
//! The shared parts (engine, settings, gamma maths, updates) build everywhere; `plat` is the
//! module for the system Luna was built for.

pub mod color;
pub mod gamma;
pub mod settings;
pub mod sharecode;
pub mod vibrance;

mod engine;
mod fit;
mod input;
mod screens;
mod sweep;
mod update;

#[cfg(windows)]
mod win;
#[cfg(windows)]
use win as plat;

#[cfg(not(windows))]
mod unix;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
use mac as plat;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as plat;

mod app {
    use crate::engine::{self, engine, Engine, EngineMsg, UiMsg};
    use crate::fit::WindowInfo;
    use crate::settings::{self, Fit, Settings};
    use crate::sharecode::{self, SharePayload};
    use crate::update::{self, UpdateStatus};
    use crate::plat::{self, display::MonitorInfo, hooks, optimizer, window::Overlay};
    use serde::Serialize;
    use std::collections::HashMap;
    use std::sync::mpsc::{channel, Sender};
    use std::sync::{Mutex, OnceLock};
    use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::{AppHandle, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};

    static ENGINE_TX: OnceLock<Mutex<Sender<EngineMsg>>> = OnceLock::new();

    fn post(msg: EngineMsg) {
        if let Some(tx) = ENGINE_TX.get() {
            if let Ok(tx) = tx.lock() {
                let _ = tx.send(msg);
            }
        }
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct AppState {
        settings: Settings,
        monitors: Vec<MonitorInfo>,
        /// At least one screen gets its vibrance from the graphics driver.
        driver_vibrance: bool,
        /// The full-screen colour matrix and Ultra Zoom work here (Windows magnifier).
        magnifier: bool,
        /// "windows", "macos" or "linux".
        platform: &'static str,
        /// The Optimizer page has something to offer here.
        optimizer: bool,
        /// Something the user should know about this system (missing permission, Wayland).
        note: Option<String>,
        gamma_strength: HashMap<String, f64>,
        version: String,
        data_dir: String,
        autostart: bool,
        update: UpdateStatus,
        /// The page to open first, when Luna restarted itself as administrator.
        start_page: Option<String>,
    }

    // ---------------- commands ----------------

    #[tauri::command]
    async fn get_state(app: AppHandle) -> AppState {
        let e = engine();
        AppState {
            settings: e.settings.clone(),
            monitors: e.monitors.clone(),
            driver_vibrance: e.driver_vibrance(),
            magnifier: plat::mag::available(),
            platform: std::env::consts::OS,
            optimizer: cfg!(windows),
            note: plat::note(),
            gamma_strength: e.gamma_strength.clone(),
            version: app.package_info().version.to_string(),
            data_dir: settings::data_dir().display().to_string(),
            autostart: plat::autostart_enabled(),
            update: update::status(),
            start_page: arg_after("--page"),
        }
    }

    #[tauri::command]
    async fn set_settings(settings: Settings) -> HashMap<String, f64> {
        let mut e = engine();
        e.set_settings(settings);
        e.gamma_strength.clone()
    }

    #[tauri::command]
    async fn reset_display() -> Settings {
        let mut e = engine();
        e.reset();
        e.settings.clone()
    }

    #[tauri::command]
    async fn start_bind_record() {
        engine().recording = true;
    }

    #[tauri::command]
    async fn cancel_bind_record() {
        engine().recording = false;
    }

    #[tauri::command]
    async fn optimizer_status() -> Vec<optimizer::TweakStatus> {
        optimizer::status()
    }

    #[tauri::command]
    async fn optimizer_set(id: String, on: bool) -> Result<Vec<optimizer::TweakStatus>, String> {
        optimizer::set(&id, on)?;
        Ok(optimizer::status())
    }

    #[derive(Serialize)]
    struct BulkResult {
        errors: Vec<String>,
        status: Vec<optimizer::TweakStatus>,
    }

    #[tauri::command]
    async fn optimizer_all() -> BulkResult {
        let errors = optimizer::optimize_all();
        BulkResult { errors, status: optimizer::status() }
    }

    #[tauri::command]
    async fn optimizer_undo_all() -> BulkResult {
        let errors = optimizer::undo_all();
        BulkResult { errors, status: optimizer::status() }
    }

    #[tauri::command]
    async fn clean_scan() -> crate::sweep::CleanReport {
        plat::cleaner::scan()
    }

    #[tauri::command]
    async fn clean_run(ids: Vec<String>) -> crate::sweep::CleanResult {
        plat::cleaner::run(&ids)
    }

    /// Starts an elevated copy of Luna, then closes this one.
    #[tauri::command]
    async fn restart_admin(app: AppHandle, page: String) -> Result<(), String> {
        plat::relaunch_admin(&page)?;
        shutdown();
        app.exit(0);
        Ok(())
    }

    #[tauri::command]
    async fn borderless_windows() -> Vec<WindowInfo> {
        plat::borderless::list()
    }

    #[tauri::command]
    async fn borderless_apply(id: i64, fit: Fit) -> Result<Vec<WindowInfo>, String> {
        let monitors = engine().monitors.clone();
        plat::borderless::apply(id, &fit, &monitors)?;
        Ok(plat::borderless::list())
    }

    #[tauri::command]
    async fn borderless_restore(id: i64) -> Result<Vec<WindowInfo>, String> {
        plat::borderless::restore(id)?;
        Ok(plat::borderless::list())
    }

    #[tauri::command]
    async fn share_encode(payload: SharePayload) -> Result<String, String> {
        sharecode::encode(&payload)
    }

    #[tauri::command]
    async fn share_decode(code: String) -> Result<SharePayload, String> {
        sharecode::decode(&code)
    }

    #[tauri::command]
    async fn set_autostart(on: bool) -> Result<bool, String> {
        plat::set_autostart(on)?;
        Ok(plat::autostart_enabled())
    }

    fn background_path() -> std::path::PathBuf {
        settings::data_dir().join("background.txt")
    }

    #[tauri::command]
    async fn get_background() -> Option<String> {
        std::fs::read_to_string(background_path()).ok()
    }

    #[tauri::command]
    async fn set_background(data: Option<String>) -> Result<(), String> {
        match data {
            Some(d) => {
                if !d.starts_with("data:image/") {
                    return Err("Only pictures can be used as a background.".into());
                }
                std::fs::create_dir_all(settings::data_dir()).map_err(|e| e.to_string())?;
                std::fs::write(background_path(), d).map_err(|e| e.to_string())
            }
            None => {
                let _ = std::fs::remove_file(background_path());
                Ok(())
            }
        }
    }

    #[tauri::command]
    async fn open_data_dir() {
        let dir = settings::data_dir();
        let _ = std::fs::create_dir_all(&dir);
        plat::open_dir(&dir);
    }

    #[tauri::command]
    async fn quit_app(app: AppHandle) {
        quit_luna(&app);
    }

    #[tauri::command]
    async fn update_status() -> UpdateStatus {
        update::status()
    }

    /// Checks now, whatever the auto-update switch says, and downloads what it finds.
    #[tauri::command]
    async fn update_check(app: AppHandle) -> UpdateStatus {
        update::check_and_download(&app).await;
        update::status()
    }

    /// Installs the downloaded update and starts Luna again.
    #[tauri::command]
    async fn update_install() -> Result<(), String> {
        shutdown();
        update::install_now(true)
    }

    /// The value that follows `flag` on the command line.
    fn arg_after(flag: &str) -> Option<String> {
        let mut args = std::env::args().skip_while(|a| a != flag);
        args.next()?;
        args.next()
    }

    fn shutdown() {
        if engine::engine_ready() {
            engine().shutdown();
        }
    }

    /// Every way out of Luna: screens back to normal, then a waiting update is installed.
    fn quit_luna(app: &AppHandle) {
        shutdown();
        update::install_pending_on_quit();
        app.exit(0);
    }

    // ---------------- windows and tray ----------------

    fn show_main(app: &AppHandle) {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.unminimize();
            let _ = w.show();
            let _ = w.set_focus();
        }
    }

    fn overlay(app: &AppHandle, label: &str, page: &str, size: f64) -> Option<Overlay> {
        let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
            .title(format!("Luna {label}"))
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .visible(false)
            .inner_size(size, size);
        // Windows gets the same through extended styles in make_overlay.
        #[cfg(not(windows))]
        let builder = builder.focusable(false).visible_on_all_workspaces(true);
        let w = builder.build().ok()?;
        let _ = w.set_ignore_cursor_events(true);
        Overlay::from_window(&w)
    }

    fn build_tray(app: &AppHandle) -> tauri::Result<()> {
        let open = MenuItem::with_id(app, "open", "Open Luna", true, None::<&str>)?;
        let tuning = CheckMenuItem::with_id(app, "tuning", "Tuning on", true, true, None::<&str>)?;
        let crosshair = CheckMenuItem::with_id(app, "crosshair", "Crosshair", true, false, None::<&str>)?;
        let reset = MenuItem::with_id(app, "reset", "Reset display", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, "quit", "Quit Luna", true, None::<&str>)?;
        let sep1 = PredefinedMenuItem::separator(app)?;
        let sep2 = PredefinedMenuItem::separator(app)?;
        let menu = Menu::with_items(app, &[&open, &sep1, &tuning, &crosshair, &reset, &sep2, &quit])?;

        let mut builder = TrayIconBuilder::with_id("luna")
            .tooltip("Luna")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id.as_ref() {
                "open" => show_main(app),
                "tuning" => post(EngineMsg::ToggleTuning),
                "crosshair" => post(EngineMsg::ToggleCrosshair),
                "reset" => post(EngineMsg::Reset),
                "quit" => quit_luna(app),
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                    show_main(tray.app_handle());
                }
            });
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone());
        }
        builder.build(app)?;

        // Keeps the check marks in step with the engine, outside the engine lock.
        let (ui_tx, ui_rx) = channel::<UiMsg>();
        let tuning_item = tuning.clone();
        let crosshair_item = crosshair.clone();
        std::thread::Builder::new()
            .name("luna-tray".into())
            .spawn(move || {
                while let Ok(UiMsg::Tray { tuning, crosshair }) = ui_rx.recv() {
                    let _ = tuning_item.set_checked(tuning);
                    let _ = crosshair_item.set_checked(crosshair);
                }
            })
            .ok();
        app.manage(TraySender(Mutex::new(ui_tx)));
        Ok(())
    }

    struct TraySender(Mutex<Sender<UiMsg>>);

    pub fn run() {
        // A panic must never leave the screens tinted.
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            engine::restore_from_file();
            engine::restore_vibrance_after_panic();
            default_hook(info);
        }));

        let autostarted = std::env::args().any(|a| a == "--autostart");
        // Restarted as administrator: let the old copy close first, or it would keep the single instance.
        if let Some(pid) = arg_after("--after").and_then(|p| p.parse().ok()) {
            plat::wait_for_exit(pid);
        }

        let app = tauri::Builder::default()
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
            .plugin(tauri_plugin_updater::Builder::new().build())
            .invoke_handler(tauri::generate_handler![
                get_state,
                set_settings,
                reset_display,
                start_bind_record,
                cancel_bind_record,
                optimizer_status,
                optimizer_set,
                optimizer_all,
                optimizer_undo_all,
                clean_scan,
                clean_run,
                restart_admin,
                borderless_windows,
                borderless_apply,
                borderless_restore,
                share_encode,
                share_decode,
                set_autostart,
                get_background,
                set_background,
                open_data_dir,
                quit_app,
                update_status,
                update_check,
                update_install
            ])
            .setup(move |app| {
                let handle = app.handle().clone();
                build_tray(&handle)?;
                let ui_tx = handle.state::<TraySender>().0.lock().map(|t| t.clone()).expect("tray channel");

                let settings = settings::load();
                let start_hidden = autostarted && settings.app.start_minimized;
                Engine::start(handle.clone(), ui_tx, settings);
                plat::borderless::watch(|| {
                    let e = engine();
                    Some((e.settings.borderless.clone(), e.monitors.clone()))
                });

                let crosshair = overlay(&handle, "crosshair", "crosshair.html", engine::CROSSHAIR_SIZE as f64);
                let wheel = overlay(&handle, "wheel", "wheel.html", engine::WHEEL_SIZE as f64);
                engine().set_overlay_windows(crosshair, wheel);

                // Engine thread: hook events, tray actions and a one-second tick.
                let (tx, rx) = channel::<EngineMsg>();
                let _ = ENGINE_TX.set(Mutex::new(tx.clone()));
                let (hook_tx, hook_rx) = channel::<hooks::HookEvent>();
                hooks::start(hook_tx);
                let fwd = tx.clone();
                std::thread::Builder::new()
                    .name("luna-hook-forward".into())
                    .spawn(move || {
                        while let Ok(e) = hook_rx.recv() {
                            if fwd.send(EngineMsg::Hook(e)).is_err() {
                                break;
                            }
                        }
                    })?;
                let tick = tx.clone();
                std::thread::Builder::new().name("luna-tick".into()).spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    if tick.send(EngineMsg::Tick).is_err() {
                        break;
                    }
                })?;
                std::thread::Builder::new().name("luna-engine".into()).spawn(move || {
                    while let Ok(msg) = rx.recv() {
                        engine().handle(msg);
                    }
                })?;

                if !start_hidden {
                    show_main(&handle);
                }
                update::start(handle.clone());
                Ok(())
            })
            .on_window_event(|window, event| {
                if window.label() != "main" {
                    return;
                }
                if let WindowEvent::Focused(on) = event {
                    crate::input::MAIN_FOCUSED.store(*on, std::sync::atomic::Ordering::Relaxed);
                }
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let to_tray = engine::engine_ready() && engine().settings.app.close_to_tray;
                    if to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        quit_luna(window.app_handle());
                    }
                }
            })
            .build(tauri::generate_context!())
            .expect("error while building Luna");

        app.run(|_app, event| match event {
            RunEvent::ExitRequested { .. } | RunEvent::Exit => shutdown(),
            _ => {}
        });
    }
}

pub fn run() {
    app::run()
}
