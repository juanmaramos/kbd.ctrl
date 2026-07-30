use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontmostApplication {
    name: Option<String>,
    bundle_id: Option<String>,
}

#[cfg(target_os = "macos")]
pub fn frontmost_bundle_id() -> Option<String> {
    use objc2_app_kit::NSWorkspace;

    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .as_ref()
        .and_then(|application| application.bundleIdentifier())
        .map(|bundle_id| bundle_id.to_string())
}

#[cfg(target_os = "macos")]
pub fn activate_codex() -> bool {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
    use objc2_foundation::ns_string;

    let applications = NSRunningApplication::runningApplicationsWithBundleIdentifier(ns_string!(
        "com.openai.codex"
    ));
    let Some(application) = applications.firstObject() else {
        return false;
    };

    application.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows)
}

#[tauri::command]
pub fn open_codex() -> bool {
    activate_codex()
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn get_frontmost_application() -> FrontmostApplication {
    use objc2_app_kit::NSWorkspace;

    let application = NSWorkspace::sharedWorkspace().frontmostApplication();

    FrontmostApplication {
        name: application
            .as_ref()
            .and_then(|application| application.localizedName())
            .map(|name| name.to_string()),
        bundle_id: application
            .as_ref()
            .and_then(|application| application.bundleIdentifier())
            .map(|bundle_id| bundle_id.to_string()),
    }
}

#[cfg(target_os = "macos")]
pub fn open_input_monitoring_settings() -> bool {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{ns_string, NSURL};

    let Some(url) = NSURL::URLWithString(ns_string!(
        "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
    )) else {
        return false;
    };

    NSWorkspace::sharedWorkspace().openURL(&url)
}

#[cfg(target_os = "macos")]
pub fn open_accessibility_settings() -> bool {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{ns_string, NSURL};

    let Some(url) = NSURL::URLWithString(ns_string!(
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
    )) else {
        return false;
    };

    NSWorkspace::sharedWorkspace().openURL(&url)
}

#[cfg(not(target_os = "macos"))]
pub fn frontmost_bundle_id() -> Option<String> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn activate_codex() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn get_frontmost_application() -> FrontmostApplication {
    FrontmostApplication {
        name: None,
        bundle_id: None,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn open_input_monitoring_settings() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
pub fn open_accessibility_settings() -> bool {
    false
}
