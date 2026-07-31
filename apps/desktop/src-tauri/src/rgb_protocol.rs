use hidapi::{HidApi, HidDevice};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

const TARGET_VENDOR_ID: u16 = 0x514c;
const TARGET_PRODUCT_ID: u16 = 0x8850;
const CONFIG_USAGE_PAGE: u16 = 0xff00;
const CONFIG_USAGE: u16 = 0x0001;
const REPORT_ID: u8 = 0x03;
const REPORT_WRITE_LENGTH: usize = 65;
const REPORT_READ_LENGTH: usize = 64;
const READ_TIMEOUT_MS: i32 = 500;
const LAYER_COUNT: u8 = 3;
const COLOR_BYTES_PER_LAYER: usize = 48;
const BUTTON_COUNT: usize = 3;
const LED_MODE_STATIC: u8 = 1;
const LED_MODE_REACTIVE: u8 = 2;
const PROFILE_STATE_FILE_NAME: &str = "rgb-profile.json";
const DEFAULT_ROLE_COLORS: [[u8; 3]; BUTTON_COUNT] = [
    [135, 78, 254], // Speak
    [255, 98, 81],  // Cancel
    [150, 211, 95], // Confirm
];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RgbLayer {
    layer: u8,
    mode: u8,
    colors: Vec<u8>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RgbBackup {
    schema_version: u8,
    captured_at_ms: u64,
    serial_number: Option<String>,
    layers: Vec<RgbLayer>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RgbStatus {
    available: bool,
    profile: Option<String>,
    modes: Vec<u8>,
    role_colors: Vec<String>,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RgbApplySummary {
    profile: &'static str,
    backup_path: String,
    verified_layers: usize,
    role_colors: Vec<String>,
}

struct OpenDevice {
    device: HidDevice,
    serial_number: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct StoredRgbProfile {
    profile: String,
    role_colors: Vec<String>,
}

impl Default for StoredRgbProfile {
    fn default() -> Self {
        Self {
            profile: String::new(),
            role_colors: role_color_hex(&DEFAULT_ROLE_COLORS),
        }
    }
}

#[tauri::command]
pub async fn get_rgb_status(app: AppHandle) -> RgbStatus {
    let fallback_app = app.clone();
    match crate::hid_service::run("lighting status", move |api| {
        Ok(get_rgb_status_blocking(app, api))
    })
    .await
    {
        Ok(status) => status,
        Err(error) => unavailable_rgb_status(&fallback_app, error),
    }
}

fn unavailable_rgb_status(app: &AppHandle, error: String) -> RgbStatus {
    let stored = read_stored_profile(app);
    RgbStatus {
        available: false,
        profile: stored.as_ref().map(|state| state.profile.clone()),
        modes: Vec::new(),
        role_colors: stored
            .map(|state| normalize_stored_role_colors(state.role_colors))
            .unwrap_or_else(|| role_color_hex(&DEFAULT_ROLE_COLORS)),
        message: error,
    }
}

fn get_rgb_status_blocking(app: AppHandle, api: &HidApi) -> RgbStatus {
    let opened = match open_configuration_interface(api) {
        Ok(opened) => opened,
        Err(_) => {
            let stored_profile = read_stored_profile(&app);
            let message = if stored_profile.is_some() {
                "This keypad revision disables key lighting over Bluetooth. Connect by USB to use or change the saved lighting profile."
            } else {
                "Connect the keypad by USB to use or configure key lighting."
            };
            return RgbStatus {
                available: false,
                profile: stored_profile.as_ref().map(|state| state.profile.clone()),
                modes: Vec::new(),
                role_colors: stored_profile
                    .map(|state| normalize_stored_role_colors(state.role_colors))
                    .unwrap_or_else(|| role_color_hex(&DEFAULT_ROLE_COLORS)),
                message: message.to_owned(),
            };
        }
    };

    match read_rgb_layers(&opened.device) {
        Ok(layers) => {
            let modes = layers.iter().map(|layer| layer.mode).collect::<Vec<_>>();
            let profile = if modes.iter().all(|mode| *mode == LED_MODE_REACTIVE) {
                Some("reactive".to_owned())
            } else if modes.iter().all(|mode| *mode == LED_MODE_STATIC) {
                Some("static".to_owned())
            } else {
                Some("custom".to_owned())
            };

            RgbStatus {
                available: true,
                profile,
                modes,
                role_colors: role_colors_from_layers(&layers),
                message:
                    "Lighting can be configured while the USB service connection is available."
                        .to_owned(),
            }
        }
        Err(error) => RgbStatus {
            available: false,
            profile: None,
            modes: Vec::new(),
            role_colors: role_color_hex(&DEFAULT_ROLE_COLORS),
            message: format!("The lighting configuration could not be read: {error}"),
        },
    }
}

#[tauri::command]
pub async fn apply_rgb_profile(
    app: AppHandle,
    profile: String,
    role_colors: Vec<String>,
) -> Result<RgbApplySummary, String> {
    crate::hid_service::run("lighting update", move |api| {
        apply_rgb_profile_blocking(app, profile, role_colors, api)
    })
    .await
}

fn apply_rgb_profile_blocking(
    app: AppHandle,
    profile: String,
    role_colors: Vec<String>,
    api: &HidApi,
) -> Result<RgbApplySummary, String> {
    let mode = match profile.as_str() {
        "reactive" => LED_MODE_REACTIVE,
        "static" => LED_MODE_STATIC,
        _ => return Err("Choose either the reactive or static lighting profile.".to_owned()),
    };
    let role_colors = parse_role_colors(&role_colors)?;

    let opened = open_configuration_interface(api)?;
    let original_layers = read_rgb_layers(&opened.device)?;
    let backup_path = save_rgb_backup(&app, opened.serial_number, &original_layers)?;

    let target_layers = original_layers
        .iter()
        .map(|layer| {
            let mut colors = layer.colors.clone();
            for (index, color) in role_colors.iter().enumerate() {
                let offset = index * 3;
                colors[offset..offset + 3].copy_from_slice(color);
            }
            RgbLayer {
                layer: layer.layer,
                mode,
                colors,
            }
        })
        .collect::<Vec<_>>();

    if let Err(write_error) = write_rgb_layers(&opened.device, &target_layers) {
        return Err(rollback_error(
            &opened.device,
            &original_layers,
            &write_error,
            &backup_path,
        ));
    }

    let verified = read_rgb_layers(&opened.device).map_err(|verification_error| {
        rollback_error(
            &opened.device,
            &original_layers,
            &verification_error,
            &backup_path,
        )
    })?;
    if verified.len() != target_layers.len()
        || verified
            .iter()
            .zip(&target_layers)
            .any(|(actual, expected)| {
                actual.layer != expected.layer
                    || actual.mode != expected.mode
                    || actual.colors != expected.colors
            })
    {
        return Err(rollback_error(
            &opened.device,
            &original_layers,
            "Lighting did not match the requested profile after read-back.",
            &backup_path,
        ));
    }

    write_stored_profile(&app, &profile, &role_colors)?;

    Ok(RgbApplySummary {
        profile: if mode == LED_MODE_REACTIVE {
            "reactive"
        } else {
            "static"
        },
        backup_path,
        verified_layers: verified.len(),
        role_colors: role_color_hex(&role_colors),
    })
}

fn write_rgb_layers(device: &HidDevice, layers: &[RgbLayer]) -> Result<(), String> {
    for layer in layers {
        write_report(device, &rgb_write_request(layer))?;
    }
    write_report(device, &commit_request())?;
    thread::sleep(Duration::from_millis(1_000));
    Ok(())
}

fn rollback_error(
    device: &HidDevice,
    original_layers: &[RgbLayer],
    cause: &str,
    backup_path: &str,
) -> String {
    match write_rgb_layers(device, original_layers) {
        Ok(()) => format!(
            "Lighting was not verified and the original profile was restored: {cause}"
        ),
        Err(restore_error) => format!(
            "Lighting failed and automatic restore also failed. Keep the keypad connected. Backup: {backup_path}. Write error: {cause}. Restore error: {restore_error}"
        ),
    }
}

fn open_configuration_interface(api: &HidApi) -> Result<OpenDevice, String> {
    let info = api
        .device_list()
        .find(|device| {
            device.vendor_id() == TARGET_VENDOR_ID
                && device.product_id() == TARGET_PRODUCT_ID
                && device.usage_page() == CONFIG_USAGE_PAGE
                && device.usage() == CONFIG_USAGE
        })
        .ok_or_else(|| {
            "Connect the keypad with its USB cable before changing lighting.".to_owned()
        })?;

    let serial_number = info.serial_number().map(ToOwned::to_owned);
    let device = info
        .open_device(api)
        .map_err(|error| format!("The RGB service connection could not be opened: {error}"))?;

    Ok(OpenDevice {
        device,
        serial_number,
    })
}

fn read_rgb_layers(device: &HidDevice) -> Result<Vec<RgbLayer>, String> {
    let mut layers = Vec::with_capacity(usize::from(LAYER_COUNT));

    for layer in 0..LAYER_COUNT {
        write_report(device, &rgb_read_request(layer))?;
        let report = read_report(device)?;
        if report.len() < 3 + COLOR_BYTES_PER_LAYER || report[..2] != [REPORT_ID, 0xfa] {
            return Err(format!(
                "Layer {} returned an unexpected RGB report.",
                layer + 1
            ));
        }

        layers.push(RgbLayer {
            layer,
            mode: report[2],
            colors: report[3..3 + COLOR_BYTES_PER_LAYER].to_vec(),
        });
    }

    Ok(layers)
}

fn save_rgb_backup(
    app: &AppHandle,
    serial_number: Option<String>,
    layers: &[RgbLayer],
) -> Result<String, String> {
    let captured_at_ms = now_ms()?;
    let backup = RgbBackup {
        schema_version: 1,
        captured_at_ms,
        serial_number: serial_number.clone(),
        layers: layers.to_vec(),
    };
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve the backup directory: {error}"))?
        .join("rgb-backups");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create the RGB backup directory: {error}"))?;
    let serial = sanitize_filename(serial_number.as_deref().unwrap_or("unknown-device"));
    let path = directory.join(format!("{serial}-{captured_at_ms}.json"));
    let bytes = serde_json::to_vec_pretty(&backup)
        .map_err(|error| format!("Could not serialize the RGB backup: {error}"))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|error| format!("Could not create the RGB backup: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("Could not write the RGB backup: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("Could not finish the RGB backup: {error}"))?;

    Ok(path.to_string_lossy().into_owned())
}

fn profile_state_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(PROFILE_STATE_FILE_NAME))
        .map_err(|error| format!("Could not resolve the settings directory: {error}"))
}

fn read_stored_profile(app: &AppHandle) -> Option<StoredRgbProfile> {
    let path = profile_state_path(app).ok()?;
    let contents = fs::read(path).ok()?;
    serde_json::from_slice::<StoredRgbProfile>(&contents).ok()
}

fn write_stored_profile(
    app: &AppHandle,
    profile: &str,
    role_colors: &[[u8; 3]; BUTTON_COUNT],
) -> Result<(), String> {
    let path = profile_state_path(app)?;
    let directory = path
        .parent()
        .ok_or_else(|| "The settings directory is unavailable.".to_owned())?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("Could not create the settings directory: {error}"))?;
    let bytes = serde_json::to_vec_pretty(&StoredRgbProfile {
        profile: profile.to_owned(),
        role_colors: role_color_hex(role_colors),
    })
    .map_err(|error| format!("Could not serialize the RGB setting: {error}"))?;
    fs::write(path, bytes).map_err(|error| format!("Could not save the RGB setting: {error}"))
}

fn rgb_read_request(layer: u8) -> [u8; REPORT_WRITE_LENGTH] {
    let mut request = [0_u8; REPORT_WRITE_LENGTH];
    request[..4].copy_from_slice(&[REPORT_ID, 0xfa, 0xb0, layer]);
    request
}

fn rgb_write_request(layer: &RgbLayer) -> [u8; REPORT_WRITE_LENGTH] {
    let mut request = [0_u8; REPORT_WRITE_LENGTH];
    request[..5].copy_from_slice(&[REPORT_ID, 0xfe, 0xb0, layer.layer, layer.mode]);
    request[5..5 + COLOR_BYTES_PER_LAYER].copy_from_slice(&layer.colors);
    request
}

fn commit_request() -> [u8; REPORT_WRITE_LENGTH] {
    let mut request = [0_u8; REPORT_WRITE_LENGTH];
    request[..4].copy_from_slice(&[REPORT_ID, 0xfd, 0xfe, 0xff]);
    request
}

fn write_report(device: &HidDevice, report: &[u8; REPORT_WRITE_LENGTH]) -> Result<(), String> {
    let written = device
        .write(report)
        .map_err(|error| format!("HID request failed: {error}"))?;
    if written != REPORT_WRITE_LENGTH {
        return Err(format!(
            "HID request was truncated: wrote {written} of {REPORT_WRITE_LENGTH} bytes."
        ));
    }
    Ok(())
}

fn read_report(device: &HidDevice) -> Result<Vec<u8>, String> {
    let mut buffer = [0_u8; REPORT_READ_LENGTH];
    let length = device
        .read_timeout(&mut buffer, READ_TIMEOUT_MS)
        .map_err(|error| format!("HID response failed: {error}"))?;
    if length == 0 {
        return Err(format!("timed out after {READ_TIMEOUT_MS} ms"));
    }
    Ok(buffer[..length].to_vec())
}

fn role_color_hex(role_colors: &[[u8; 3]; BUTTON_COUNT]) -> Vec<String> {
    role_colors
        .iter()
        .map(|color| format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]))
        .collect()
}

fn role_colors_from_layers(layers: &[RgbLayer]) -> Vec<String> {
    let Some(layer) = layers.first() else {
        return role_color_hex(&DEFAULT_ROLE_COLORS);
    };
    if layer.colors.len() < BUTTON_COUNT * 3 {
        return role_color_hex(&DEFAULT_ROLE_COLORS);
    }

    (0..BUTTON_COUNT)
        .map(|index| {
            let offset = index * 3;
            format!(
                "#{:02X}{:02X}{:02X}",
                layer.colors[offset],
                layer.colors[offset + 1],
                layer.colors[offset + 2]
            )
        })
        .collect()
}

fn normalize_stored_role_colors(mut values: Vec<String>) -> Vec<String> {
    values.truncate(BUTTON_COUNT);
    if values.len() == BUTTON_COUNT {
        values
    } else {
        role_color_hex(&DEFAULT_ROLE_COLORS)
    }
}

fn parse_role_colors(values: &[String]) -> Result<[[u8; 3]; BUTTON_COUNT], String> {
    if values.len() != BUTTON_COUNT {
        return Err("Choose one color for Speak, Cancel, and Confirm.".to_owned());
    }

    let mut colors = [[0_u8; 3]; BUTTON_COUNT];
    for (index, value) in values.iter().enumerate() {
        let value = value
            .strip_prefix('#')
            .ok_or_else(|| format!("Color {} must use #RRGGBB format.", index + 1))?;
        if value.len() != 6 {
            return Err(format!("Color {} must use #RRGGBB format.", index + 1));
        }
        for channel in 0..3 {
            colors[index][channel] =
                u8::from_str_radix(&value[channel * 2..channel * 2 + 2], 16)
                    .map_err(|_| format!("Color {} must use #RRGGBB format.", index + 1))?;
        }
    }
    Ok(colors)
}

fn now_ms() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .map_err(|error| format!("System clock error: {error}"))
}

fn sanitize_filename(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();

    if sanitized.is_empty() {
        "unknown-device".to_owned()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_vendor_rgb_read_request() {
        let request = rgb_read_request(2);
        assert_eq!(&request[..5], &[0x03, 0xfa, 0xb0, 0x02, 0x00]);
    }

    #[test]
    fn builds_vendor_rgb_write_request() {
        let colors = (0..COLOR_BYTES_PER_LAYER)
            .map(|value| value as u8)
            .collect::<Vec<_>>();
        let request = rgb_write_request(&RgbLayer {
            layer: 1,
            mode: LED_MODE_REACTIVE,
            colors: colors.clone(),
        });

        assert_eq!(&request[..5], &[0x03, 0xfe, 0xb0, 0x01, 0x02]);
        assert_eq!(&request[5..53], colors.as_slice());
        assert!(request[53..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn role_colors_match_physical_control_order() {
        assert_eq!(
            role_color_hex(&DEFAULT_ROLE_COLORS),
            vec!["#874EFE", "#FF6251", "#96D35F"]
        );
    }

    #[test]
    fn parses_custom_role_colors() {
        let colors = parse_role_colors(&[
            "#10A37F".to_owned(),
            "#FF0000".to_owned(),
            "#00FF00".to_owned(),
        ])
        .unwrap();

        assert_eq!(colors[0], [16, 163, 127]);
        assert_eq!(colors[2], [0, 255, 0]);
    }

    #[test]
    fn ignores_legacy_fourth_color_for_unlit_dial_slot() {
        assert_eq!(
            normalize_stored_role_colors(vec![
                "#874EFE".to_owned(),
                "#FF6251".to_owned(),
                "#96D35F".to_owned(),
                "#3B82F6".to_owned(),
            ]),
            vec!["#874EFE", "#FF6251", "#96D35F"]
        );
    }
}
