use hidapi::{HidApi, HidDevice};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    thread,
    time::{SystemTime, UNIX_EPOCH},
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
const EXPECTED_KEY_COUNT: u8 = 3;
const EXPECTED_KNOB_COUNT: u8 = 1;
const EXPECTED_LAYOUT_CODE: u8 = 0x0b;
const COMMIT_DELAY_MS: u64 = 1_000;
const FAILED_CONTROL_F13_MODE: u8 = 0x02;
const FAILED_LEFT_CONTROL: u8 = 0xf1;

#[derive(Clone, Copy)]
struct TargetMapping {
    record_index: u8,
    usage: u8,
    label: &'static str,
}

const TARGET_MAPPINGS: [TargetMapping; 6] = [
    TargetMapping {
        record_index: 1,
        usage: 0x68,
        label: "left key → F13",
    },
    TargetMapping {
        record_index: 2,
        usage: 0x6b,
        label: "middle key → F16",
    },
    TargetMapping {
        record_index: 3,
        usage: 0x6c,
        label: "right key → F17",
    },
    TargetMapping {
        record_index: 16,
        usage: 0x6d,
        label: "knob left → F18",
    },
    TargetMapping {
        record_index: 18,
        usage: 0x6e,
        label: "knob right → F19",
    },
    TargetMapping {
        record_index: 17,
        usage: 0x6f,
        label: "knob press → F20",
    },
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceBackupSummary {
    path: String,
    fingerprint_sha256: String,
    report_count: usize,
    captured_at_ms: u64,
    device_info: Vec<u8>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingSummary {
    prewrite_backup_path: String,
    verification_fingerprint_sha256: String,
    written_report_count: usize,
    mappings: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingStatus {
    configured: bool,
    message: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceBackup {
    schema_version: u8,
    read_only_capture: bool,
    captured_at_ms: u64,
    device: BackupDevice,
    protocol: BackupProtocol,
    fingerprint_sha256: String,
    device_info_report: String,
    layers: Vec<LayerBackup>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupDevice {
    vendor_id: String,
    product_id: String,
    product: Option<String>,
    serial_number: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupProtocol {
    source: String,
    report_id: u8,
    report_write_length: usize,
    report_read_length: usize,
    key_count: u8,
    knob_count: u8,
    layout_code: u8,
    read_strategy: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LayerBackup {
    layer: u8,
    request: String,
    reports: Vec<String>,
}

struct OpenDevice {
    device: HidDevice,
    product: Option<String>,
    serial_number: Option<String>,
}

#[tauri::command]
pub async fn backup_device_configuration(app: AppHandle) -> Result<DeviceBackupSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _transaction = crate::lock_hid_transaction()?;
        backup_device_configuration_blocking(app)
    })
    .await
    .map_err(|error| format!("The backup task could not finish: {error}"))?
}

fn backup_device_configuration_blocking(app: AppHandle) -> Result<DeviceBackupSummary, String> {
    let opened = open_configuration_interface()?;
    let info_report = request_device_info(&opened.device)?;
    let info_bytes = info_report[2..5].to_vec();
    let is_compact_device = info_report[4] < 10;
    let expected_reports = if is_compact_device { 24 } else { 25 };
    let mut layers = Vec::with_capacity(usize::from(LAYER_COUNT));
    let mut fingerprint_input = info_report.clone();

    for layer in 1..=LAYER_COUNT {
        let request = configuration_read_request(layer, is_compact_device);
        write_report(&opened.device, &request)?;

        let mut reports = Vec::with_capacity(expected_reports);
        for report_index in 1..=expected_reports {
            let report = read_report(&opened.device).map_err(|error| {
                format!(
                    "Layer {layer} report {report_index}/{expected_reports} could not be read: {error}"
                )
            })?;
            fingerprint_input.extend_from_slice(&report);
            reports.push(hex(&report));
        }

        layers.push(LayerBackup {
            layer,
            request: hex(&request[..5]),
            reports,
        });
    }

    let captured_at_ms = now_ms()?;
    let fingerprint_sha256 = format!("{:x}", Sha256::digest(&fingerprint_input));
    let backup = DeviceBackup {
        schema_version: 1,
        read_only_capture: true,
        captured_at_ms,
        device: BackupDevice {
            vendor_id: format!("0x{TARGET_VENDOR_ID:04x}"),
            product_id: format!("0x{TARGET_PRODUCT_ID:04x}"),
            product: opened.product,
            serial_number: opened.serial_number.clone(),
        },
        protocol: BackupProtocol {
            source: "statically verified vendor macOS configurator".to_owned(),
            report_id: REPORT_ID,
            report_write_length: REPORT_WRITE_LENGTH,
            report_read_length: REPORT_READ_LENGTH,
            key_count: info_report[2],
            knob_count: info_report[3],
            layout_code: info_report[4],
            read_strategy: if is_compact_device {
                "15 key records + 9 knob records per layer".to_owned()
            } else {
                "25 records per layer".to_owned()
            },
        },
        fingerprint_sha256: fingerprint_sha256.clone(),
        device_info_report: hex(&info_report),
        layers,
    };

    let backup_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve the backup directory: {error}"))?
        .join("backups");
    fs::create_dir_all(&backup_dir)
        .map_err(|error| format!("Could not create the backup directory: {error}"))?;

    let serial = sanitize_filename(opened.serial_number.as_deref().unwrap_or("unknown-device"));
    let path = backup_dir.join(format!("{serial}-{captured_at_ms}.json"));
    let bytes = serde_json::to_vec_pretty(&backup)
        .map_err(|error| format!("Could not serialize the backup: {error}"))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|error| format!("Could not create the backup file: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("Could not write the backup file: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("Could not finish the backup file: {error}"))?;

    Ok(DeviceBackupSummary {
        path: path.to_string_lossy().into_owned(),
        fingerprint_sha256,
        report_count: 1 + expected_reports * usize::from(LAYER_COUNT),
        captured_at_ms,
        device_info: info_bytes,
    })
}

#[tauri::command]
pub async fn configure_transport_mapping(
    app: AppHandle,
    backup_path: String,
) -> Result<MappingSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _transaction = crate::lock_hid_transaction()?;
        configure_transport_mapping_blocking(app, backup_path)
    })
    .await
    .map_err(|error| format!("The mapping task could not finish: {error}"))?
}

fn configure_transport_mapping_blocking(
    app: AppHandle,
    backup_path: String,
) -> Result<MappingSummary, String> {
    let backup = load_verified_backup(&app, &backup_path)?;
    validate_target_device(&backup)?;

    let opened = open_configuration_interface()?;
    let live_info = request_device_info(&opened.device)?;
    let backed_up_info = parse_hex(&backup.device_info_report)?;
    if live_info != backed_up_info {
        return Err(
            "The connected device no longer matches the selected backup. Create a fresh backup."
                .to_owned(),
        );
    }

    let write_result = write_target_mapping(&opened.device, &backup.layers);
    if let Err(write_error) = write_result {
        return Err(rollback_error(&opened.device, &backup.layers, &write_error));
    }

    thread::sleep(std::time::Duration::from_millis(COMMIT_DELAY_MS));
    let verified_layers = read_layers(&opened.device, false).map_err(|verification_error| {
        rollback_error(&opened.device, &backup.layers, &verification_error)
    })?;

    if let Err(verification_error) = verify_target_mapping(&verified_layers) {
        return Err(rollback_error(
            &opened.device,
            &backup.layers,
            &verification_error,
        ));
    }

    let mut fingerprint_input = live_info;
    for layer in &verified_layers {
        for report in &layer.reports {
            fingerprint_input
                .extend_from_slice(&parse_hex(report).map_err(|error| {
                    format!("Could not fingerprint verification data: {error}")
                })?);
        }
    }

    Ok(MappingSummary {
        prewrite_backup_path: backup_path,
        verification_fingerprint_sha256: format!("{:x}", Sha256::digest(&fingerprint_input)),
        written_report_count: TARGET_MAPPINGS.len() * usize::from(LAYER_COUNT) + 1,
        mappings: TARGET_MAPPINGS
            .iter()
            .map(|mapping| mapping.label)
            .collect(),
    })
}

#[tauri::command]
pub async fn inspect_transport_mapping() -> Result<MappingStatus, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let _transaction = crate::lock_hid_transaction()?;
        inspect_transport_mapping_blocking()
    })
    .await
    .map_err(|error| format!("The mapping inspection could not finish: {error}"))?
}

fn inspect_transport_mapping_blocking() -> Result<MappingStatus, String> {
    let opened = open_configuration_interface()?;
    let info_report = request_device_info(&opened.device)?;
    if info_report.len() < 5
        || info_report[2] != EXPECTED_KEY_COUNT
        || info_report[3] != EXPECTED_KNOB_COUNT
        || info_report[4] != EXPECTED_LAYOUT_CODE
    {
        return Err(format!(
            "Mapping inspection supports only the verified 3-key + 1-knob layout 0x{EXPECTED_LAYOUT_CODE:02x}."
        ));
    }

    let is_compact_device = info_report[4] < 10;
    let layers = read_layers(&opened.device, is_compact_device)?;
    let configured = verify_target_mapping(&layers).is_ok();

    Ok(MappingStatus {
        configured,
        message: if configured {
            "The universal F13 and F16–F20 mapping is already verified.".to_owned()
        } else {
            "This keypad still needs the universal mapping.".to_owned()
        },
    })
}

fn open_configuration_interface() -> Result<OpenDevice, String> {
    let api = HidApi::new().map_err(|error| format!("HID initialization failed: {error}"))?;
    let info = api
        .device_list()
        .find(|device| {
            device.vendor_id() == TARGET_VENDOR_ID
                && device.product_id() == TARGET_PRODUCT_ID
                && device.usage_page() == CONFIG_USAGE_PAGE
                && device.usage() == CONFIG_USAGE
        })
        .ok_or_else(|| "The vendor configuration interface is not visible.".to_owned())?;

    let product = info.product_string().map(ToOwned::to_owned);
    let serial_number = info.serial_number().map(ToOwned::to_owned);
    let device = info.open_device(&api).map_err(|error| {
        format!(
            "The configuration interface could not be opened. Confirm kbd.ctrl has Input Monitoring access, then reconnect the keypad. HID error: {error}"
        )
    })?;

    Ok(OpenDevice {
        device,
        product,
        serial_number,
    })
}

fn request_device_info(device: &HidDevice) -> Result<Vec<u8>, String> {
    let request = device_info_request();
    write_report(device, &request)?;
    let report = read_report(device)?;

    if report.len() < 5 {
        return Err(format!(
            "The device-info response was only {} bytes; at least 5 were expected.",
            report.len()
        ));
    }

    Ok(report)
}

fn read_layers(device: &HidDevice, is_compact_device: bool) -> Result<Vec<LayerBackup>, String> {
    let expected_reports = if is_compact_device { 24 } else { 25 };
    let mut layers = Vec::with_capacity(usize::from(LAYER_COUNT));

    for layer in 1..=LAYER_COUNT {
        let request = configuration_read_request(layer, is_compact_device);
        write_report(device, &request)?;

        let mut reports = Vec::with_capacity(expected_reports);
        for report_index in 1..=expected_reports {
            let report = read_report(device).map_err(|error| {
                format!(
                    "Layer {layer} report {report_index}/{expected_reports} could not be read: {error}"
                )
            })?;
            reports.push(hex(&report));
        }

        layers.push(LayerBackup {
            layer,
            request: hex(&request[..5]),
            reports,
        });
    }

    Ok(layers)
}

fn load_verified_backup(app: &AppHandle, backup_path: &str) -> Result<DeviceBackup, String> {
    let backup_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve the backup directory: {error}"))?
        .join("backups")
        .canonicalize()
        .map_err(|error| format!("Could not access the backup directory: {error}"))?;
    let path = Path::new(backup_path)
        .canonicalize()
        .map_err(|error| format!("Could not access the selected backup: {error}"))?;
    if !path.starts_with(&backup_dir) {
        return Err("The selected file is outside kbd.ctrl's backup directory.".to_owned());
    }

    let bytes =
        fs::read(&path).map_err(|error| format!("Could not read the selected backup: {error}"))?;
    let backup: DeviceBackup = serde_json::from_slice(&bytes)
        .map_err(|error| format!("The selected backup is invalid: {error}"))?;

    let expected_fingerprint = fingerprint_backup(&backup)?;
    if expected_fingerprint != backup.fingerprint_sha256 {
        return Err("The selected backup fingerprint does not match its contents.".to_owned());
    }
    Ok(backup)
}

fn validate_target_device(backup: &DeviceBackup) -> Result<(), String> {
    if backup.schema_version != 1
        || !backup.read_only_capture
        || backup.device.vendor_id != format!("0x{TARGET_VENDOR_ID:04x}")
        || backup.device.product_id != format!("0x{TARGET_PRODUCT_ID:04x}")
    {
        return Err("This is not a compatible kbd.ctrl backup.".to_owned());
    }
    if backup.protocol.key_count != EXPECTED_KEY_COUNT
        || backup.protocol.knob_count != EXPECTED_KNOB_COUNT
        || backup.protocol.layout_code != EXPECTED_LAYOUT_CODE
        || backup.layers.len() != usize::from(LAYER_COUNT)
    {
        return Err(format!(
            "Mapping is restricted to the verified 3-key + 1-knob layout 0x{EXPECTED_LAYOUT_CODE:02x}."
        ));
    }
    Ok(())
}

fn write_target_mapping(device: &HidDevice, layers: &[LayerBackup]) -> Result<(), String> {
    for layer in layers {
        for mapping in TARGET_MAPPINGS {
            let report = find_report(layer, mapping.record_index)?;
            validate_mappable_key_report(&report, layer.layer, mapping)?;
            let request = mapping_write_request(report, mapping)?;
            write_report(device, &request)?;
        }
    }
    write_report(device, &commit_request())
}

fn restore_target_mapping(device: &HidDevice, layers: &[LayerBackup]) -> Result<(), String> {
    for layer in layers {
        for mapping in TARGET_MAPPINGS {
            let report = find_report(layer, mapping.record_index)?;
            let request = write_request_from_read_report(report, None)?;
            write_report(device, &request)?;
        }
    }
    write_report(device, &commit_request())?;
    thread::sleep(std::time::Duration::from_millis(COMMIT_DELAY_MS));
    Ok(())
}

fn rollback_error(device: &HidDevice, layers: &[LayerBackup], cause: &str) -> String {
    match restore_target_mapping(device, layers) {
        Ok(()) => format!("Mapping was not verified and the backup was restored: {cause}"),
        Err(restore_error) => format!(
            "Mapping failed and automatic restore also failed. Do not disconnect the keypad. Mapping error: {cause}. Restore error: {restore_error}"
        ),
    }
}

fn verify_target_mapping(layers: &[LayerBackup]) -> Result<(), String> {
    for layer in layers {
        for mapping in TARGET_MAPPINGS {
            let report = find_report(layer, mapping.record_index)?;
            validate_report_header(&report, layer.layer, mapping.record_index)?;
            verify_mapping_report(&report, layer.layer, mapping)?;
        }
    }
    Ok(())
}

fn find_report(layer: &LayerBackup, record_index: u8) -> Result<Vec<u8>, String> {
    for encoded in &layer.reports {
        let report = parse_hex(encoded)?;
        if report.get(2) == Some(&record_index) {
            return Ok(report);
        }
    }
    Err(format!(
        "Layer {} is missing record {record_index}.",
        layer.layer
    ))
}

fn validate_report_header(report: &[u8], layer: u8, record_index: u8) -> Result<(), String> {
    if report.len() < 61
        || report[..2] != [REPORT_ID, 0xfa]
        || report[2] != record_index
        || report[3] != layer
    {
        return Err(format!(
            "Layer {layer} record {record_index} has an unexpected report header."
        ));
    }
    Ok(())
}

fn validate_mappable_key_report(
    report: &[u8],
    layer: u8,
    mapping: TargetMapping,
) -> Result<(), String> {
    validate_report_header(report, layer, mapping.record_index)?;

    if matches_target_mapping(report, mapping) {
        return Ok(());
    }

    if mapping.record_index == 1 && matches_failed_control_f13_mapping(report) {
        return Ok(());
    }

    if report[4..9] != [0x01, 0x01, 0x01, 0x00, 0x00] {
        return Err(format!(
            "Layer {layer} record {} is neither a single basic key nor the verified target mapping.",
            mapping.record_index
        ));
    }
    Ok(())
}

fn mapping_write_request(
    report: Vec<u8>,
    mapping: TargetMapping,
) -> Result<[u8; REPORT_WRITE_LENGTH], String> {
    if report[4..9] == [0x01, 0x01, 0x01, 0x00, 0x00] {
        return write_request_from_read_report(report, Some(mapping.usage));
    }

    let mut request = write_request_from_read_report(report, None)?;
    request[4..61].fill(0);
    request[4..9].copy_from_slice(&[0x01, 0x01, 0x01, 0x00, 0x00]);
    request[9] = mapping.usage;
    Ok(request)
}

fn verify_mapping_report(report: &[u8], layer: u8, mapping: TargetMapping) -> Result<(), String> {
    if matches_target_mapping(report, mapping) {
        return Ok(());
    }

    Err(format!(
        "Layer {layer} record {} did not read back as {}. Returned bytes: {}.",
        mapping.record_index,
        mapping.label,
        hex(&report[..report.len().min(20)])
    ))
}

fn matches_target_mapping(report: &[u8], mapping: TargetMapping) -> bool {
    report[4..9] == [0x01, 0x01, 0x01, 0x00, 0x00] && report[9] == mapping.usage
}

fn matches_failed_control_f13_mapping(report: &[u8]) -> bool {
    report[4..13]
        == [
            FAILED_CONTROL_F13_MODE,
            0x01,
            0x02,
            0x00,
            0x00,
            FAILED_LEFT_CONTROL,
            0x00,
            0x00,
            0x68,
        ]
}

fn write_request_from_read_report(
    report: Vec<u8>,
    replacement_usage: Option<u8>,
) -> Result<[u8; REPORT_WRITE_LENGTH], String> {
    if report.len() < 61 {
        return Err("A backed-up record is too short to restore.".to_owned());
    }

    let mut request = [0_u8; REPORT_WRITE_LENGTH];
    request[0] = REPORT_ID;
    request[1] = 0xfd;
    request[2..61].copy_from_slice(&report[2..61]);
    if let Some(usage) = replacement_usage {
        request[9] = usage;
    }
    Ok(request)
}

fn commit_request() -> [u8; REPORT_WRITE_LENGTH] {
    let mut request = [0_u8; REPORT_WRITE_LENGTH];
    request[..4].copy_from_slice(&[REPORT_ID, 0xfd, 0xfe, 0xff]);
    request
}

fn fingerprint_backup(backup: &DeviceBackup) -> Result<String, String> {
    let mut input = parse_hex(&backup.device_info_report)?;
    for layer in &backup.layers {
        for report in &layer.reports {
            input.extend_from_slice(&parse_hex(report)?);
        }
    }
    Ok(format!("{:x}", Sha256::digest(input)))
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

fn device_info_request() -> [u8; REPORT_WRITE_LENGTH] {
    let mut request = [0_u8; REPORT_WRITE_LENGTH];
    request[..4].copy_from_slice(&[REPORT_ID, 0xfb, 0xfb, 0xfb]);
    request
}

fn configuration_read_request(layer: u8, is_compact_device: bool) -> [u8; REPORT_WRITE_LENGTH] {
    let mut request = [0_u8; REPORT_WRITE_LENGTH];
    let (primary_count, auxiliary_count) = if is_compact_device {
        (0x0f, 0x03)
    } else {
        (0x19, 0x00)
    };
    request[..5].copy_from_slice(&[REPORT_ID, 0xfa, primary_count, auxiliary_count, layer]);
    request
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

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_hex(value: &str) -> Result<Vec<u8>, String> {
    value
        .split_whitespace()
        .map(|part| {
            u8::from_str_radix(part, 16)
                .map_err(|error| format!("Invalid backup byte {part:?}: {error}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_vendor_device_info_request() {
        let request = device_info_request();
        assert_eq!(&request[..5], &[0x03, 0xfb, 0xfb, 0xfb, 0x00]);
        assert!(request[4..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn builds_compact_configuration_read_request() {
        let request = configuration_read_request(2, true);
        assert_eq!(&request[..6], &[0x03, 0xfa, 0x0f, 0x03, 0x02, 0x00]);
        assert!(request[5..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn builds_extended_configuration_read_request() {
        let request = configuration_read_request(3, false);
        assert_eq!(&request[..6], &[0x03, 0xfa, 0x19, 0x00, 0x03, 0x00]);
        assert!(request[5..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn builds_basic_key_write_from_vendor_read_report() {
        let mut report = vec![0_u8; REPORT_READ_LENGTH];
        report[..10].copy_from_slice(&[0x03, 0xfa, 0x01, 0x01, 0x01, 0x01, 0x01, 0, 0, 0x04]);
        let request = write_request_from_read_report(report, Some(0x68)).unwrap();
        assert_eq!(
            &request[..10],
            &[0x03, 0xfd, 0x01, 0x01, 0x01, 0x01, 0x01, 0, 0, 0x68]
        );
    }

    #[test]
    fn migrates_failed_control_f13_record_back_to_basic_f13() {
        let mut report = vec![0_u8; REPORT_READ_LENGTH];
        report[..13].copy_from_slice(&[
            0x03,
            0xfa,
            0x01,
            0x01,
            FAILED_CONTROL_F13_MODE,
            0x01,
            0x02,
            0x00,
            0x00,
            FAILED_LEFT_CONTROL,
            0x00,
            0x00,
            0x68,
        ]);

        let request = mapping_write_request(report, TARGET_MAPPINGS[0]).unwrap();

        assert_eq!(
            &request[..10],
            &[0x03, 0xfd, 0x01, 0x01, 0x01, 0x01, 0x01, 0, 0, 0x68]
        );
        assert!(request[10..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn builds_vendor_commit_request() {
        let request = commit_request();
        assert_eq!(&request[..5], &[0x03, 0xfd, 0xfe, 0xff, 0x00]);
    }

    #[test]
    fn transport_mapping_avoids_macos_brightness_keys() {
        let usages = TARGET_MAPPINGS
            .iter()
            .map(|mapping| mapping.usage)
            .collect::<Vec<_>>();

        assert_eq!(usages, vec![0x68, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f]);
        assert!(!usages.contains(&0x69));
        assert!(!usages.contains(&0x6a));
    }

    #[test]
    fn sanitizes_backup_filename_components() {
        assert_eq!(sanitize_filename("A/B C"), "A-B-C");
        assert_eq!(sanitize_filename(""), "unknown-device");
    }
}
