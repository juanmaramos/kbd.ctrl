mod app_context;
mod app_settings;
#[cfg(target_os = "macos")]
mod claude_accessibility;
mod config_protocol;
mod input_monitor;
mod rgb_protocol;

use hidapi::HidApi;
use input_monitor::InputMonitor;
use serde::Serialize;
use std::sync::{Mutex, MutexGuard};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_autostart::MacosLauncher;

const TARGET_VENDOR_ID: u16 = 0x514c;
const TARGET_PRODUCT_ID: u16 = 0x8850;
const CONFIG_USAGE_PAGE: u16 = 0xff00;
const CONFIG_USAGE: u16 = 0x0001;
const TRAY_ICON_RGBA: &[u8] = include_bytes!("../icons/tray-dice.rgba");
static HID_TRANSACTION: Mutex<()> = Mutex::new(());

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceStatus {
    connected: bool,
    product: Option<String>,
    serial_number: Option<String>,
    vendor_id: String,
    product_id: String,
    configuration_interface_visible: bool,
    keyboard_interface_visible: bool,
    input_monitoring_granted: bool,
    control_access_granted: bool,
    error: Option<String>,
}

pub(crate) fn lock_hid_transaction() -> Result<MutexGuard<'static, ()>, String> {
    HID_TRANSACTION
        .lock()
        .map_err(|_| "The HID transaction lock is unavailable. Relaunch kbd.ctrl.".to_owned())
}

#[cfg(target_os = "macos")]
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOHIDCheckAccess(request_type: i32) -> i32;
    fn IOHIDRequestAccess(request_type: i32) -> bool;
}

#[cfg(target_os = "macos")]
const IOHID_REQUEST_TYPE_LISTEN_EVENT: i32 = 1;

#[cfg(target_os = "macos")]
const IOHID_ACCESS_TYPE_GRANTED: i32 = 0;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

#[cfg(target_os = "macos")]
fn has_input_monitoring_access() -> bool {
    unsafe { IOHIDCheckAccess(IOHID_REQUEST_TYPE_LISTEN_EVENT) == IOHID_ACCESS_TYPE_GRANTED }
}

#[cfg(target_os = "macos")]
fn request_input_monitoring() -> bool {
    unsafe { IOHIDRequestAccess(IOHID_REQUEST_TYPE_LISTEN_EVENT) }
}

#[cfg(target_os = "macos")]
pub(crate) fn has_control_access() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

#[cfg(not(target_os = "macos"))]
fn has_input_monitoring_access() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
fn request_input_monitoring() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn has_control_access() -> bool {
    true
}

fn unavailable_device_status(error: String) -> DeviceStatus {
    DeviceStatus {
        connected: false,
        product: None,
        serial_number: None,
        vendor_id: format!("0x{TARGET_VENDOR_ID:04x}"),
        product_id: format!("0x{TARGET_PRODUCT_ID:04x}"),
        configuration_interface_visible: false,
        keyboard_interface_visible: false,
        input_monitoring_granted: has_input_monitoring_access(),
        control_access_granted: has_control_access(),
        error: Some(error),
    }
}

fn get_device_status_blocking() -> DeviceStatus {
    let mut status = DeviceStatus {
        connected: false,
        product: None,
        serial_number: None,
        vendor_id: format!("0x{TARGET_VENDOR_ID:04x}"),
        product_id: format!("0x{TARGET_PRODUCT_ID:04x}"),
        configuration_interface_visible: false,
        keyboard_interface_visible: false,
        input_monitoring_granted: has_input_monitoring_access(),
        control_access_granted: has_control_access(),
        error: None,
    };

    let api = match HidApi::new() {
        Ok(api) => api,
        Err(error) => {
            status.error = Some(error.to_string());
            return status;
        }
    };

    for device in api.device_list().filter(|device| {
        device.vendor_id() == TARGET_VENDOR_ID && device.product_id() == TARGET_PRODUCT_ID
    }) {
        status.connected = true;
        status.product = device.product_string().map(ToOwned::to_owned);
        status.serial_number = device.serial_number().map(ToOwned::to_owned);

        if device.usage_page() == CONFIG_USAGE_PAGE && device.usage() == CONFIG_USAGE {
            status.configuration_interface_visible = true;
        }

        if device.usage_page() == 0x0001 && device.usage() == 0x0006 {
            status.keyboard_interface_visible = true;
        }
    }

    status
}

#[tauri::command]
async fn get_device_status() -> DeviceStatus {
    match tauri::async_runtime::spawn_blocking(|| {
        let _transaction = lock_hid_transaction()?;
        Ok::<_, String>(get_device_status_blocking())
    })
    .await
    {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => unavailable_device_status(error),
        Err(error) => {
            unavailable_device_status(format!("Device status task could not finish: {error}"))
        }
    }
}

#[tauri::command]
fn request_input_monitoring_access() -> bool {
    let granted = request_input_monitoring();
    if !granted {
        app_context::open_input_monitoring_settings();
    }
    granted
}

#[tauri::command]
fn request_control_access() -> bool {
    let granted = has_control_access();
    if !granted {
        app_context::open_accessibility_settings();
    }
    granted
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(InputMonitor::default())
        .setup(|app| {
            app.handle().plugin(tauri_plugin_autostart::init(
                MacosLauncher::LaunchAgent,
                Some(vec!["--background"]),
            ))?;

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let status_item =
                MenuItem::with_id(app, "status", "kbd.ctrl active", false, None::<&str>)?;
            let open_item = MenuItem::with_id(app, "open", "Open kbd.ctrl", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit kbd.ctrl", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&status_item, &open_item, &separator, &quit_item])?;

            let tray = TrayIconBuilder::with_id("main")
                .icon(Image::new(TRAY_ICON_RGBA, 36, 36))
                .icon_as_template(true)
                .tooltip("kbd.ctrl")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            tray.set_visible(true)?;
            app.manage(tray);
            app_settings::apply_saved_dock_preference(app.handle());

            if std::env::args().any(|argument| argument == "--background") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_context::get_frontmost_application,
            app_context::open_codex,
            app_settings::get_app_preferences,
            app_settings::set_launch_at_login,
            app_settings::set_onboarding_state,
            app_settings::set_show_dock_icon,
            config_protocol::backup_device_configuration,
            config_protocol::configure_transport_mapping,
            config_protocol::inspect_transport_mapping,
            get_device_status,
            request_input_monitoring_access,
            request_control_access,
            input_monitor::start_input_monitor,
            input_monitor::stop_input_monitor,
            input_monitor::test_codex_transport,
            rgb_protocol::apply_rgb_profile,
            rgb_protocol::get_rgb_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
