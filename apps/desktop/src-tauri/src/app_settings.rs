use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    show_dock_icon: bool,
    launch_at_login: bool,
    onboarding: OnboardingState,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OnboardingState {
    completed: bool,
    dismissed: bool,
    hardware_configured: bool,
    codex_configured: bool,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct StoredPreferences {
    show_dock_icon: bool,
    onboarding: OnboardingState,
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(SETTINGS_FILE_NAME))
        .map_err(|error| error.to_string())
}

fn read_stored_preferences(app: &AppHandle) -> StoredPreferences {
    let Ok(path) = settings_path(app) else {
        return StoredPreferences::default();
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return StoredPreferences::default();
    };

    serde_json::from_str(&contents).unwrap_or_default()
}

fn write_stored_preferences(
    app: &AppHandle,
    preferences: &StoredPreferences,
) -> Result<(), String> {
    let path = settings_path(app)?;
    let directory = path
        .parent()
        .ok_or_else(|| "The settings directory is unavailable.".to_owned())?;
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let contents = serde_json::to_vec_pretty(preferences).map_err(|error| error.to_string())?;
    fs::write(path, contents).map_err(|error| error.to_string())
}

pub fn apply_saved_dock_preference(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let preferences = read_stored_preferences(app);
        let _ = app.set_dock_visibility(preferences.show_dock_icon);
    }
}

#[tauri::command]
pub fn get_app_preferences(app: AppHandle) -> Result<AppPreferences, String> {
    let stored = read_stored_preferences(&app);
    let launch_at_login = app
        .autolaunch()
        .is_enabled()
        .map_err(|error| error.to_string())?;

    Ok(AppPreferences {
        show_dock_icon: stored.show_dock_icon,
        launch_at_login,
        onboarding: stored.onboarding,
    })
}

#[tauri::command]
pub fn set_show_dock_icon(app: AppHandle, visible: bool) -> Result<AppPreferences, String> {
    #[cfg(target_os = "macos")]
    app.set_dock_visibility(visible)
        .map_err(|error| error.to_string())?;

    let mut stored = read_stored_preferences(&app);
    stored.show_dock_icon = visible;
    write_stored_preferences(&app, &stored)?;
    get_app_preferences(app)
}

#[tauri::command]
pub fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<AppPreferences, String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|error| error.to_string())?;

    get_app_preferences(app)
}

#[tauri::command]
pub fn set_onboarding_state(
    app: AppHandle,
    onboarding: OnboardingState,
) -> Result<AppPreferences, String> {
    let mut stored = read_stored_preferences(&app);
    stored.onboarding = onboarding;
    write_stored_preferences(&app, &stored)?;
    get_app_preferences(app)
}

#[cfg(test)]
mod tests {
    use super::StoredPreferences;

    #[test]
    fn older_settings_files_start_with_incomplete_onboarding() {
        let preferences: StoredPreferences =
            serde_json::from_str(r#"{"showDockIcon":true}"#).unwrap();

        assert!(preferences.show_dock_icon);
        assert!(!preferences.onboarding.completed);
        assert!(!preferences.onboarding.dismissed);
        assert!(!preferences.onboarding.hardware_configured);
        assert!(!preferences.onboarding.codex_configured);
    }
}
