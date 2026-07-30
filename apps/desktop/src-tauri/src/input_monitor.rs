#[cfg(not(target_os = "macos"))]
use hidapi::{HidApi, HidDevice};
use serde::Serialize;
#[cfg(not(target_os = "macos"))]
use std::collections::BTreeSet;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, State};

#[cfg(target_os = "macos")]
use core_foundation::runloop::{kCFRunLoopDefaultMode, CFRunLoop};
#[cfg(target_os = "macos")]
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, CallbackResult, EventField,
};
#[cfg(target_os = "macos")]
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
#[cfg(target_os = "macos")]
use core_graphics::geometry::CGPoint;

#[cfg(not(target_os = "macos"))]
const TARGET_VENDOR_ID: u16 = 0x514c;
#[cfg(not(target_os = "macos"))]
const TARGET_PRODUCT_ID: u16 = 0x8850;
const INPUT_EVENT_NAME: &str = "kbd-input";
const MONITOR_STATUS_EVENT_NAME: &str = "kbd-monitor-status";
#[cfg(target_os = "macos")]
const CODEX_BUNDLE_ID: &str = "com.openai.codex";
#[cfg(target_os = "macos")]
const CLAUDE_EXPERIMENTAL_ENABLED: bool = false;
#[cfg(target_os = "macos")]
const MACOS_F13_KEY_CODE: i64 = 0x69;
#[cfg(target_os = "macos")]
const MACOS_F16_KEY_CODE: i64 = 0x6a;
#[cfg(target_os = "macos")]
const MACOS_F17_KEY_CODE: i64 = 0x40;
#[cfg(target_os = "macos")]
const MACOS_F18_KEY_CODE: i64 = 0x4f;
#[cfg(target_os = "macos")]
const MACOS_F19_KEY_CODE: i64 = 0x50;
#[cfg(target_os = "macos")]
const MACOS_F20_KEY_CODE: i64 = 0x5a;
#[cfg(target_os = "macos")]
const MACOS_RETURN_KEY_CODE: i64 = 0x24;
#[cfg(target_os = "macos")]
const MACOS_ESCAPE_KEY_CODE: i64 = 0x35;
#[cfg(target_os = "macos")]
const MACOS_M_KEY_CODE: i64 = 0x2e;
#[cfg(target_os = "macos")]
const MACOS_E_KEY_CODE: i64 = 0x0e;
#[cfg(target_os = "macos")]
const MACOS_I_KEY_CODE: i64 = 0x22;

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
struct ClaudeVoiceGesture {
    target: crate::claude_accessibility::VoiceTarget,
    cursor: CGPoint,
}

#[derive(Default)]
pub struct InputMonitor {
    generation: Arc<AtomicU64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MonitorStatus {
    state: &'static str,
    interface_count: usize,
    message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InputEvent {
    sequence: u64,
    timestamp_ms: u64,
    control: String,
    state: &'static str,
    usage_page: String,
    usage: String,
    value: Option<i32>,
    duration_ms: Option<u64>,
    modifiers: Vec<String>,
    report_id: Option<u8>,
    raw: String,
}

#[cfg(not(target_os = "macos"))]
struct Reader {
    device: HidDevice,
    usage_page: u16,
    usage: u16,
    previous_keyboard_usages: BTreeSet<u8>,
    previous_buttons: u8,
    previous_consumer_usage: u16,
    pressed_at: HashMap<(u16, u16), Instant>,
}

#[cfg(not(target_os = "macos"))]
impl Reader {
    fn new(device: HidDevice, usage_page: u16, usage: u16) -> Self {
        Self {
            device,
            usage_page,
            usage,
            previous_keyboard_usages: BTreeSet::new(),
            previous_buttons: 0,
            previous_consumer_usage: 0,
            pressed_at: HashMap::new(),
        }
    }

    fn decode(&mut self, bytes: &[u8], sequence: &AtomicU64) -> Vec<InputEvent> {
        if bytes.is_empty() {
            return Vec::new();
        }

        match bytes[0] {
            1 | 4 => self.decode_keyboard(bytes, sequence),
            2 => self.decode_pointer(bytes, sequence),
            5 => self.decode_consumer(bytes, sequence),
            _ => match (self.usage_page, self.usage) {
                (0x01, 0x06) => self.decode_keyboard(bytes, sequence),
                (0x01, 0x02) => self.decode_pointer(bytes, sequence),
                (0x0c, _) => self.decode_consumer(bytes, sequence),
                _ => vec![self.event(
                    sequence,
                    "raw_report".to_owned(),
                    "report",
                    self.usage_page,
                    self.usage,
                    None,
                    None,
                    Vec::new(),
                    None,
                    bytes,
                )],
            },
        }
    }

    fn decode_keyboard(&mut self, bytes: &[u8], sequence: &AtomicU64) -> Vec<InputEvent> {
        let (report_id, payload) = split_report(bytes, &[1, 4]);
        if payload.len() < 3 {
            return vec![self.raw_event(sequence, report_id, bytes)];
        }

        let modifiers = modifier_names(payload[0]);
        let current = payload[2..]
            .iter()
            .copied()
            .filter(|usage| *usage != 0)
            .collect::<BTreeSet<_>>();
        let pressed = current
            .difference(&self.previous_keyboard_usages)
            .copied()
            .collect::<Vec<_>>();
        let released = self
            .previous_keyboard_usages
            .difference(&current)
            .copied()
            .collect::<Vec<_>>();
        let mut events = Vec::with_capacity(pressed.len() + released.len());

        for usage in pressed {
            self.pressed_at
                .insert((0x07, u16::from(usage)), Instant::now());
            events.push(self.event(
                sequence,
                keyboard_label(usage),
                "down",
                0x07,
                u16::from(usage),
                None,
                None,
                modifiers.clone(),
                report_id,
                bytes,
            ));
        }

        for usage in released {
            let duration_ms = self.take_duration(0x07, u16::from(usage));
            events.push(self.event(
                sequence,
                keyboard_label(usage),
                "up",
                0x07,
                u16::from(usage),
                None,
                duration_ms,
                modifiers.clone(),
                report_id,
                bytes,
            ));
        }

        self.previous_keyboard_usages = current;
        events
    }

    fn decode_pointer(&mut self, bytes: &[u8], sequence: &AtomicU64) -> Vec<InputEvent> {
        let (report_id, payload) = split_report(bytes, &[2]);
        if payload.len() < 4 {
            return vec![self.raw_event(sequence, report_id, bytes)];
        }

        let buttons = payload[0] & 0x07;
        let changed_buttons = self.previous_buttons ^ buttons;
        let mut events = Vec::new();

        for index in 0..3 {
            let mask = 1_u8 << index;
            if changed_buttons & mask == 0 {
                continue;
            }

            let usage = (index + 1) as u16;
            let is_down = buttons & mask != 0;
            let duration_ms = if is_down {
                self.pressed_at.insert((0x09, usage), Instant::now());
                None
            } else {
                self.take_duration(0x09, usage)
            };

            events.push(self.event(
                sequence,
                format!("pointer_button_{}", index + 1),
                if is_down { "down" } else { "up" },
                0x09,
                usage,
                None,
                duration_ms,
                Vec::new(),
                report_id,
                bytes,
            ));
        }

        let wheel = i32::from(payload[3] as i8);
        if wheel != 0 {
            events.push(self.event(
                sequence,
                if wheel > 0 {
                    "wheel_up".to_owned()
                } else {
                    "wheel_down".to_owned()
                },
                "step",
                0x01,
                0x38,
                Some(wheel),
                None,
                Vec::new(),
                report_id,
                bytes,
            ));
        }

        self.previous_buttons = buttons;
        events
    }

    fn decode_consumer(&mut self, bytes: &[u8], sequence: &AtomicU64) -> Vec<InputEvent> {
        let (report_id, payload) = split_report(bytes, &[5]);
        if payload.len() < 2 {
            return vec![self.raw_event(sequence, report_id, bytes)];
        }

        let current = u16::from(payload[0]) | (u16::from(payload[1]) << 8);
        let mut events = Vec::new();

        if self.previous_consumer_usage != 0 && current != self.previous_consumer_usage {
            let previous = self.previous_consumer_usage;
            let duration_ms = self.take_duration(0x0c, previous);
            events.push(self.event(
                sequence,
                consumer_label(previous),
                "up",
                0x0c,
                previous,
                None,
                duration_ms,
                Vec::new(),
                report_id,
                bytes,
            ));
        }

        if current != 0 && current != self.previous_consumer_usage {
            self.pressed_at.insert((0x0c, current), Instant::now());
            events.push(self.event(
                sequence,
                consumer_label(current),
                "down",
                0x0c,
                current,
                None,
                None,
                Vec::new(),
                report_id,
                bytes,
            ));
        }

        self.previous_consumer_usage = current;
        events
    }

    fn raw_event(&self, sequence: &AtomicU64, report_id: Option<u8>, bytes: &[u8]) -> InputEvent {
        self.event(
            sequence,
            "raw_report".to_owned(),
            "report",
            self.usage_page,
            self.usage,
            None,
            None,
            Vec::new(),
            report_id,
            bytes,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn event(
        &self,
        sequence: &AtomicU64,
        control: String,
        state: &'static str,
        usage_page: u16,
        usage: u16,
        value: Option<i32>,
        duration_ms: Option<u64>,
        modifiers: Vec<String>,
        report_id: Option<u8>,
        bytes: &[u8],
    ) -> InputEvent {
        make_event(
            sequence,
            control,
            state,
            format!("0x{usage_page:02x}"),
            format!("0x{usage:02x}"),
            value,
            duration_ms,
            modifiers,
            report_id,
            hex(bytes),
        )
    }

    fn take_duration(&mut self, usage_page: u16, usage: u16) -> Option<u64> {
        self.pressed_at
            .remove(&(usage_page, usage))
            .map(|started| started.elapsed().as_millis() as u64)
    }
}

#[tauri::command]
pub fn start_input_monitor(app: AppHandle, monitor: State<'_, InputMonitor>) -> Result<(), String> {
    let generation = Arc::clone(&monitor.generation);
    let token = generation.fetch_add(1, Ordering::SeqCst) + 1;

    thread::Builder::new()
        .name("kbd-input-monitor".to_owned())
        .spawn(move || monitor_loop(app, generation, token))
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn stop_input_monitor(monitor: State<'_, InputMonitor>) {
    monitor.generation.fetch_add(1, Ordering::SeqCst);
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn test_codex_transport(control: String) -> Result<(), String> {
    let key_code = match control.as_str() {
        "F18" => MACOS_F18_KEY_CODE,
        "F19" => MACOS_F19_KEY_CODE,
        "F20" => MACOS_F20_KEY_CODE,
        _ => return Err("Only F18, F19, and F20 can be tested.".to_owned()),
    };

    if !crate::has_control_access() {
        return Err("Control access is required for virtual testing.".to_owned());
    }
    if !crate::app_context::activate_codex() {
        return Err("Codex is not running.".to_owned());
    }

    let activation_deadline = Instant::now() + Duration::from_secs(2);
    while crate::app_context::frontmost_bundle_id().as_deref() != Some(CODEX_BUNDLE_ID) {
        if Instant::now() >= activation_deadline {
            return Err("Codex did not become the active application.".to_owned());
        }
        thread::sleep(Duration::from_millis(50));
    }

    thread::sleep(Duration::from_millis(100));
    post_virtual_key(key_code)
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn test_codex_transport(_control: String) -> Result<(), String> {
    Err("Virtual Codex testing is currently available only on macOS.".to_owned())
}

#[cfg(target_os = "macos")]
fn post_virtual_key(key_code: i64) -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "Could not create a virtual keyboard event source.".to_owned())?;
    let key_code = u16::try_from(key_code)
        .map_err(|_| "The virtual key code is outside the macOS range.".to_owned())?;
    let down = CGEvent::new_keyboard_event(source.clone(), key_code, true)
        .map_err(|_| "Could not create the virtual key-down event.".to_owned())?;
    let up = CGEvent::new_keyboard_event(source, key_code, false)
        .map_err(|_| "Could not create the virtual key-up event.".to_owned())?;

    down.post(CGEventTapLocation::HID);
    thread::sleep(Duration::from_millis(40));
    up.post(CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "macos")]
fn monitor_loop(app: AppHandle, generation: Arc<AtomicU64>, token: u64) {
    monitor_event_tap(app, generation, token, Vec::new());
}

#[cfg(not(target_os = "macos"))]
fn monitor_loop(app: AppHandle, generation: Arc<AtomicU64>, token: u64) {
    emit_status(&app, "starting", 0, "Opening device input interfaces");

    let api = match HidApi::new() {
        Ok(api) => api,
        Err(error) => {
            emit_status(&app, "error", 0, &error.to_string());
            return;
        }
    };

    let interfaces = api
        .device_list()
        .filter(|device| {
            device.vendor_id() == TARGET_VENDOR_ID
                && device.product_id() == TARGET_PRODUCT_ID
                && is_input_collection(device.usage_page(), device.usage())
        })
        .map(|device| {
            (
                device.path().to_owned(),
                device.usage_page(),
                device.usage(),
            )
        })
        .collect::<Vec<_>>();

    let mut readers = Vec::new();
    let mut open_errors = Vec::new();

    for (path, usage_page, usage) in interfaces {
        match api.open_path(path.as_c_str()) {
            Ok(device) => readers.push(Reader::new(device, usage_page, usage)),
            Err(error) => {
                open_errors.push(format!("{usage_page:#06x}:{usage:#06x} — {error}"));
            }
        }
    }

    if readers.is_empty() {
        let message = if open_errors.is_empty() {
            "No keyboard, pointer, or consumer-control collection was found".to_owned()
        } else {
            format!("Could not open HID input: {}", open_errors.join(" · "))
        };
        emit_status(&app, "error", 0, &message);
        return;
    }

    emit_status(
        &app,
        "listening",
        readers.len(),
        "Listening to this keypad only",
    );

    let sequence = AtomicU64::new(0);
    let mut buffer = [0_u8; 64];

    while generation.load(Ordering::SeqCst) == token {
        let mut index = 0;
        while index < readers.len() {
            match readers[index].device.read_timeout(&mut buffer, 12) {
                Ok(0) => index += 1,
                Ok(length) => {
                    for event in readers[index].decode(&buffer[..length], &sequence) {
                        let _ = app.emit(INPUT_EVENT_NAME, event);
                    }
                    index += 1;
                }
                Err(error) => {
                    emit_status(
                        &app,
                        "warning",
                        readers.len().saturating_sub(1),
                        &format!("An input collection closed: {error}"),
                    );
                    readers.remove(index);
                }
            }
        }

        if readers.is_empty() {
            emit_status(&app, "error", 0, "All input collections are closed");
            return;
        }

        thread::sleep(Duration::from_millis(2));
    }

    if generation.load(Ordering::SeqCst) == token {
        emit_status(&app, "stopped", 0, "Input monitoring stopped");
    }
}

#[cfg(target_os = "macos")]
fn monitor_event_tap(
    app: AppHandle,
    generation: Arc<AtomicU64>,
    token: u64,
    open_errors: Vec<String>,
) {
    use std::sync::Mutex;

    let callback_app = app.clone();
    let sequence = Arc::new(AtomicU64::new(0));
    let callback_sequence = Arc::clone(&sequence);
    let pressed_at = Arc::new(Mutex::new(HashMap::<i64, Instant>::new()));
    let callback_pressed_at = Arc::clone(&pressed_at);
    let routed_keys = Arc::new(Mutex::new(HashMap::<i64, (i64, CGEventFlags)>::new()));
    let callback_routed_keys = Arc::clone(&routed_keys);
    let custom_keys = Arc::new(Mutex::new(HashSet::<i64>::new()));
    let callback_custom_keys = Arc::clone(&custom_keys);
    let claude_voice_gesture = Arc::new(Mutex::new(None::<ClaudeVoiceGesture>));
    let callback_claude_voice_gesture = Arc::clone(&claude_voice_gesture);
    let can_translate = crate::has_control_access();

    let result = CGEventTap::with_enabled(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        if can_translate {
            CGEventTapOptions::Default
        } else {
            CGEventTapOptions::ListenOnly
        },
        vec![CGEventType::KeyDown, CGEventType::KeyUp],
        move |_proxy, event_type, event| {
            if let Some(input_event) =
                decode_macos_event(event_type, event, &callback_sequence, &callback_pressed_at)
            {
                let _ = callback_app.emit(INPUT_EVENT_NAME, input_event);
            }

            if !can_translate {
                return CallbackResult::Keep;
            }

            if CLAUDE_EXPERIMENTAL_ENABLED {
                if let Some(result) = route_claude_control(
                    event_type,
                    event,
                    &callback_routed_keys,
                    &callback_custom_keys,
                    &callback_claude_voice_gesture,
                ) {
                    return result;
                }
            }

            if let Some(destination) = controller_global_destination(
                event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE),
                event.get_flags(),
            ) {
                let translated = event.clone();
                translated
                    .set_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE, destination.0);
                translated.set_flags(destination.1);
                CallbackResult::Replace(translated)
            } else {
                route_codex_control(event_type, event, &callback_routed_keys)
                    .map(CallbackResult::Replace)
                    .unwrap_or(CallbackResult::Keep)
            }
        },
        || {
            let detail = if open_errors.is_empty() {
                ""
            } else {
                " Direct HID reports are protected by macOS."
            };
            emit_status(
                &app,
                "listening",
                0,
                &format!(
                    "{}{detail}",
                    if can_translate {
                        "Controller input and Codex translations are ready."
                    } else {
                        "Listening for F13 and F16–F20. Grant Control access to enable app profiles."
                    }
                ),
            );

            while generation.load(Ordering::SeqCst) == token {
                CFRunLoop::run_in_mode(
                    unsafe { kCFRunLoopDefaultMode },
                    Duration::from_millis(50),
                    true,
                );
            }
        },
    );

    if result.is_err() && generation.load(Ordering::SeqCst) == token {
        emit_status(
            &app,
            "error",
            0,
            if can_translate {
                "macOS denied the active shortcut router. Re-enable Control access and relaunch."
            } else {
                "macOS denied the passive event listener. Re-enable Input Monitoring and relaunch."
            },
        );
    } else if generation.load(Ordering::SeqCst) == token {
        emit_status(&app, "stopped", 0, "Input monitoring stopped");
    }
}

#[cfg(target_os = "macos")]
fn controller_global_destination(
    source_key_code: i64,
    flags: CGEventFlags,
) -> Option<(i64, CGEventFlags)> {
    let modifier_flags = CGEventFlags::CGEventFlagControl
        | CGEventFlags::CGEventFlagShift
        | CGEventFlags::CGEventFlagAlternate
        | CGEventFlags::CGEventFlagCommand;

    if source_key_code == MACOS_F13_KEY_CODE && !flags.intersects(modifier_flags) {
        Some((MACOS_F13_KEY_CODE, flags | CGEventFlags::CGEventFlagControl))
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn is_bare_key(event: &CGEvent, key_code: i64) -> bool {
    event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) == key_code
        && !event.get_flags().intersects(
            CGEventFlags::CGEventFlagControl
                | CGEventFlags::CGEventFlagShift
                | CGEventFlags::CGEventFlagAlternate
                | CGEventFlags::CGEventFlagCommand,
        )
}

#[cfg(target_os = "macos")]
fn route_codex_control(
    event_type: CGEventType,
    event: &CGEvent,
    routed_keys: &std::sync::Mutex<HashMap<i64, (i64, CGEventFlags)>>,
) -> Option<CGEvent> {
    let source_key_code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);

    let destination = match event_type {
        CGEventType::KeyDown => {
            let mut routed_keys = routed_keys.lock().ok()?;
            if let Some(destination) = routed_keys.get(&source_key_code) {
                Some(*destination)
            } else if is_bare_key(event, source_key_code)
                && crate::app_context::frontmost_bundle_id().as_deref() == Some(CODEX_BUNDLE_ID)
            {
                let destination = codex_destination(source_key_code)?;
                routed_keys.insert(source_key_code, destination);
                Some(destination)
            } else {
                None
            }
        }
        CGEventType::KeyUp => routed_keys.lock().ok()?.remove(&source_key_code),
        _ => None,
    }?;

    let translated = event.clone();
    translated.set_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE, destination.0);
    translated.set_flags(destination.1);
    Some(translated)
}

#[cfg(target_os = "macos")]
fn codex_destination(source_key_code: i64) -> Option<(i64, CGEventFlags)> {
    match source_key_code {
        MACOS_F16_KEY_CODE => Some((MACOS_ESCAPE_KEY_CODE, CGEventFlags::empty())),
        MACOS_F17_KEY_CODE => Some((MACOS_RETURN_KEY_CODE, CGEventFlags::empty())),
        MACOS_F18_KEY_CODE => Some((MACOS_F18_KEY_CODE, CGEventFlags::empty())),
        MACOS_F19_KEY_CODE => Some((MACOS_F19_KEY_CODE, CGEventFlags::empty())),
        MACOS_F20_KEY_CODE => Some((
            MACOS_M_KEY_CODE,
            CGEventFlags::CGEventFlagControl | CGEventFlags::CGEventFlagShift,
        )),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn route_claude_control(
    event_type: CGEventType,
    event: &CGEvent,
    routed_keys: &std::sync::Mutex<HashMap<i64, (i64, CGEventFlags)>>,
    custom_keys: &std::sync::Mutex<HashSet<i64>>,
    voice_gesture: &std::sync::Mutex<Option<ClaudeVoiceGesture>>,
) -> Option<CallbackResult> {
    use crate::claude_accessibility::{ClaudeSurface, EffortDirection, CLAUDE_BUNDLE_ID};

    let source_key_code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);

    match event_type {
        CGEventType::KeyUp => {
            if source_key_code == MACOS_F13_KEY_CODE {
                if let Some(gesture) = voice_gesture.lock().ok()?.take() {
                    let _ =
                        crate::claude_accessibility::post_voice_mouse_event(gesture.target, false);
                    let _ = crate::claude_accessibility::restore_pointer(gesture.cursor);
                }
                if custom_keys.lock().ok()?.remove(&source_key_code) {
                    return Some(CallbackResult::Drop);
                }
                return None;
            }

            if custom_keys.lock().ok()?.remove(&source_key_code) {
                return Some(CallbackResult::Drop);
            }

            if let Some(destination) = routed_keys.lock().ok()?.remove(&source_key_code) {
                return Some(CallbackResult::Replace(translated_event(
                    event,
                    destination,
                )));
            }

            return None;
        }
        CGEventType::KeyDown => {}
        _ => return None,
    }

    if !is_bare_key(event, source_key_code) {
        return None;
    }

    if let Some(destination) = routed_keys.lock().ok()?.get(&source_key_code).copied() {
        return Some(CallbackResult::Replace(translated_event(
            event,
            destination,
        )));
    }
    if custom_keys.lock().ok()?.contains(&source_key_code) {
        return Some(CallbackResult::Drop);
    }

    if crate::app_context::frontmost_bundle_id().as_deref() != Some(CLAUDE_BUNDLE_ID) {
        return None;
    }

    let surface = crate::claude_accessibility::surface();

    if source_key_code == MACOS_F13_KEY_CODE {
        custom_keys.lock().ok()?.insert(source_key_code);
        if surface == ClaudeSurface::Chat {
            if let (Ok(target), Ok(cursor)) = (
                crate::claude_accessibility::voice_target(),
                crate::claude_accessibility::current_pointer_location(),
            ) {
                if crate::claude_accessibility::post_voice_mouse_event(target, true).is_ok() {
                    *voice_gesture.lock().ok()? = Some(ClaudeVoiceGesture { target, cursor });
                }
            }
        }
        return Some(CallbackResult::Drop);
    }

    if surface == ClaudeSurface::Chat
        && matches!(
            source_key_code,
            MACOS_F18_KEY_CODE | MACOS_F19_KEY_CODE | MACOS_F20_KEY_CODE
        )
    {
        custom_keys.lock().ok()?.insert(source_key_code);
        thread::spawn(move || {
            let _ = match source_key_code {
                MACOS_F18_KEY_CODE => {
                    crate::claude_accessibility::adjust_chat_effort(EffortDirection::Decrease)
                }
                MACOS_F19_KEY_CODE => {
                    crate::claude_accessibility::adjust_chat_effort(EffortDirection::Increase)
                }
                MACOS_F20_KEY_CODE => crate::claude_accessibility::open_chat_model_picker(),
                _ => Ok(()),
            };
        });
        return Some(CallbackResult::Drop);
    }

    let destination = claude_destination(surface, source_key_code)?;
    routed_keys
        .lock()
        .ok()?
        .insert(source_key_code, destination);
    Some(CallbackResult::Replace(translated_event(
        event,
        destination,
    )))
}

#[cfg(target_os = "macos")]
fn translated_event(event: &CGEvent, destination: (i64, CGEventFlags)) -> CGEvent {
    let translated = event.clone();
    translated.set_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE, destination.0);
    translated.set_flags(destination.1);
    translated
}

#[cfg(target_os = "macos")]
fn claude_destination(
    surface: crate::claude_accessibility::ClaudeSurface,
    source_key_code: i64,
) -> Option<(i64, CGEventFlags)> {
    use crate::claude_accessibility::ClaudeSurface;

    match source_key_code {
        MACOS_F16_KEY_CODE => Some((MACOS_ESCAPE_KEY_CODE, CGEventFlags::empty())),
        MACOS_F17_KEY_CODE => Some((MACOS_RETURN_KEY_CODE, CGEventFlags::empty())),
        MACOS_F18_KEY_CODE | MACOS_F19_KEY_CODE if surface == ClaudeSurface::Code => Some((
            MACOS_E_KEY_CODE,
            CGEventFlags::CGEventFlagCommand | CGEventFlags::CGEventFlagShift,
        )),
        MACOS_F20_KEY_CODE if surface == ClaudeSurface::Code => Some((
            MACOS_I_KEY_CODE,
            CGEventFlags::CGEventFlagCommand | CGEventFlags::CGEventFlagShift,
        )),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn decode_macos_event(
    event_type: CGEventType,
    event: &CGEvent,
    sequence: &AtomicU64,
    pressed_at: &std::sync::Mutex<HashMap<i64, Instant>>,
) -> Option<InputEvent> {
    match event_type {
        CGEventType::KeyDown => {
            if event.get_integer_value_field(EventField::KEYBOARD_EVENT_AUTOREPEAT) != 0 {
                return None;
            }

            let key_code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
            let label = controller_macos_key_label(key_code)?;
            if !begin_key_press(pressed_at, key_code) {
                return None;
            }
            Some(macos_key_event(
                sequence, event, key_code, label, "down", None,
            ))
        }
        CGEventType::KeyUp => {
            let key_code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
            let label = controller_macos_key_label(key_code)?;
            let duration_ms = finish_key_press(pressed_at, key_code)?;
            Some(macos_key_event(
                sequence,
                event,
                key_code,
                label,
                "up",
                Some(duration_ms),
            ))
        }
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn begin_key_press(pressed_at: &std::sync::Mutex<HashMap<i64, Instant>>, key_code: i64) -> bool {
    let Ok(mut pressed_at) = pressed_at.lock() else {
        return false;
    };
    if pressed_at.contains_key(&key_code) {
        return false;
    }
    pressed_at.insert(key_code, Instant::now());
    true
}

#[cfg(target_os = "macos")]
fn finish_key_press(
    pressed_at: &std::sync::Mutex<HashMap<i64, Instant>>,
    key_code: i64,
) -> Option<u64> {
    pressed_at
        .lock()
        .ok()?
        .remove(&key_code)
        .map(|started| started.elapsed().as_millis() as u64)
}

#[cfg(target_os = "macos")]
fn macos_key_event(
    sequence: &AtomicU64,
    event: &CGEvent,
    key_code: i64,
    label: &'static str,
    state: &'static str,
    duration_ms: Option<u64>,
) -> InputEvent {
    make_event(
        sequence,
        label.to_owned(),
        state,
        "macos".to_owned(),
        format!("keycode_0x{key_code:02x}"),
        None,
        duration_ms,
        macos_modifiers(event.get_flags()),
        None,
        format!("CGEvent keyCode {key_code}"),
    )
}

#[cfg(target_os = "macos")]
fn controller_macos_key_label(key_code: i64) -> Option<&'static str> {
    match key_code {
        0x40 => Some("F17"),
        0x4f => Some("F18"),
        0x50 => Some("F19"),
        0x5a => Some("F20"),
        0x69 => Some("F13"),
        0x6a => Some("F16"),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn macos_modifiers(flags: CGEventFlags) -> Vec<String> {
    [
        (CGEventFlags::CGEventFlagControl, "control"),
        (CGEventFlags::CGEventFlagShift, "shift"),
        (CGEventFlags::CGEventFlagAlternate, "option"),
        (CGEventFlags::CGEventFlagCommand, "command"),
        (CGEventFlags::CGEventFlagSecondaryFn, "fn"),
    ]
    .into_iter()
    .filter(|(flag, _)| flags.contains(*flag))
    .map(|(_, name)| name.to_owned())
    .collect()
}

#[cfg(not(target_os = "macos"))]
fn is_input_collection(usage_page: u16, usage: u16) -> bool {
    matches!((usage_page, usage), (0x01, 0x02 | 0x06)) || usage_page == 0x0c
}

#[cfg(not(target_os = "macos"))]
fn split_report<'a>(bytes: &'a [u8], report_ids: &[u8]) -> (Option<u8>, &'a [u8]) {
    match bytes.first().copied() {
        Some(report_id) if report_ids.contains(&report_id) => (Some(report_id), &bytes[1..]),
        _ => (None, bytes),
    }
}

#[cfg(not(target_os = "macos"))]
fn keyboard_label(usage: u8) -> String {
    match usage {
        0x04..=0x1d => char::from(b'a' + usage - 0x04).to_string(),
        0x1e..=0x26 => char::from(b'1' + usage - 0x1e).to_string(),
        0x27 => "0".to_owned(),
        0x28 => "enter".to_owned(),
        0x29 => "escape".to_owned(),
        0x2a => "backspace".to_owned(),
        0x2b => "tab".to_owned(),
        0x2c => "space".to_owned(),
        0x3a..=0x45 => format!("F{}", usage - 0x3a + 1),
        0x68..=0x73 => format!("F{}", usage - 0x68 + 13),
        _ => format!("keyboard_usage_0x{usage:02x}"),
    }
}

#[cfg(not(target_os = "macos"))]
fn consumer_label(usage: u16) -> String {
    match usage {
        0x00b5 => "next_track".to_owned(),
        0x00b6 => "previous_track".to_owned(),
        0x00cd => "play_pause".to_owned(),
        0x00e2 => "mute".to_owned(),
        0x00e9 => "volume_up".to_owned(),
        0x00ea => "volume_down".to_owned(),
        _ => format!("consumer_usage_0x{usage:04x}"),
    }
}

#[cfg(not(target_os = "macos"))]
fn modifier_names(byte: u8) -> Vec<String> {
    [
        "left_control",
        "left_shift",
        "left_option",
        "left_command",
        "right_control",
        "right_shift",
        "right_option",
        "right_command",
    ]
    .iter()
    .enumerate()
    .filter(|(index, _)| byte & (1 << index) != 0)
    .map(|(_, name)| (*name).to_owned())
    .collect()
}

fn emit_status(app: &AppHandle, state: &'static str, interface_count: usize, message: &str) {
    let _ = app.emit(
        MONITOR_STATUS_EVENT_NAME,
        MonitorStatus {
            state,
            interface_count,
            message: message.to_owned(),
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn make_event(
    sequence: &AtomicU64,
    control: String,
    state: &'static str,
    usage_page: String,
    usage: String,
    value: Option<i32>,
    duration_ms: Option<u64>,
    modifiers: Vec<String>,
    report_id: Option<u8>,
    raw: String,
) -> InputEvent {
    InputEvent {
        sequence: sequence.fetch_add(1, Ordering::Relaxed) + 1,
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        control,
        state,
        usage_page,
        usage,
        value,
        duration_ms,
        modifiers,
        report_id,
        raw,
    }
}

#[cfg(not(target_os = "macos"))]
fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::{
        begin_key_press, claude_destination, codex_destination, controller_global_destination,
        controller_macos_key_label, finish_key_press, CGEventFlags, MACOS_ESCAPE_KEY_CODE,
        MACOS_E_KEY_CODE, MACOS_F13_KEY_CODE, MACOS_F16_KEY_CODE, MACOS_F17_KEY_CODE,
        MACOS_F18_KEY_CODE, MACOS_F19_KEY_CODE, MACOS_F20_KEY_CODE, MACOS_I_KEY_CODE,
        MACOS_M_KEY_CODE, MACOS_RETURN_KEY_CODE,
    };
    use crate::claude_accessibility::ClaudeSurface;
    use std::{collections::HashMap, sync::Mutex};

    #[test]
    fn global_monitor_accepts_only_controller_transport_keys() {
        assert_eq!(controller_macos_key_label(0x69), Some("F13"));
        assert_eq!(controller_macos_key_label(0x6a), Some("F16"));
        assert_eq!(controller_macos_key_label(0x40), Some("F17"));
        assert_eq!(controller_macos_key_label(0x4f), Some("F18"));
        assert_eq!(controller_macos_key_label(0x50), Some("F19"));
        assert_eq!(controller_macos_key_label(0x5a), Some("F20"));

        assert_eq!(controller_macos_key_label(0x00), None);
        assert_eq!(controller_macos_key_label(0x24), None);
        assert_eq!(controller_macos_key_label(0x6b), None);
        assert_eq!(controller_macos_key_label(0x71), None);
    }

    #[test]
    fn codex_profile_routes_confirm_and_cancel_keys() {
        assert_eq!(
            codex_destination(MACOS_F16_KEY_CODE),
            Some((MACOS_ESCAPE_KEY_CODE, CGEventFlags::empty()))
        );
        assert_eq!(
            codex_destination(MACOS_F17_KEY_CODE),
            Some((MACOS_RETURN_KEY_CODE, CGEventFlags::empty()))
        );
        assert_eq!(
            codex_destination(MACOS_F18_KEY_CODE),
            Some((MACOS_F18_KEY_CODE, CGEventFlags::empty()))
        );
        assert_eq!(
            codex_destination(MACOS_F19_KEY_CODE),
            Some((MACOS_F19_KEY_CODE, CGEventFlags::empty()))
        );
        assert_eq!(
            codex_destination(MACOS_F20_KEY_CODE),
            Some((
                MACOS_M_KEY_CODE,
                CGEventFlags::CGEventFlagControl | CGEventFlags::CGEventFlagShift
            ))
        );
        assert_eq!(codex_destination(0x69), None);
    }

    #[test]
    fn global_profile_adds_control_to_bare_f13_only() {
        assert_eq!(
            controller_global_destination(MACOS_F13_KEY_CODE, CGEventFlags::empty()),
            Some((MACOS_F13_KEY_CODE, CGEventFlags::CGEventFlagControl))
        );
        assert_eq!(
            controller_global_destination(MACOS_F13_KEY_CODE, CGEventFlags::CGEventFlagShift),
            None
        );
        assert_eq!(
            controller_global_destination(MACOS_F16_KEY_CODE, CGEventFlags::empty()),
            None
        );
    }

    #[test]
    fn repeated_key_down_and_unmatched_key_up_are_ignored() {
        let pressed = Mutex::new(HashMap::new());

        assert!(begin_key_press(&pressed, MACOS_F16_KEY_CODE));
        assert!(!begin_key_press(&pressed, MACOS_F16_KEY_CODE));
        assert!(finish_key_press(&pressed, MACOS_F16_KEY_CODE).is_some());
        assert_eq!(finish_key_press(&pressed, MACOS_F16_KEY_CODE), None);
    }

    #[test]
    fn claude_chat_routes_only_context_safe_keyboard_actions() {
        assert_eq!(
            claude_destination(ClaudeSurface::Chat, MACOS_F16_KEY_CODE),
            Some((MACOS_ESCAPE_KEY_CODE, CGEventFlags::empty()))
        );
        assert_eq!(
            claude_destination(ClaudeSurface::Chat, MACOS_F17_KEY_CODE),
            Some((MACOS_RETURN_KEY_CODE, CGEventFlags::empty()))
        );
        assert_eq!(
            claude_destination(ClaudeSurface::Chat, MACOS_F18_KEY_CODE),
            None
        );
        assert_eq!(
            claude_destination(ClaudeSurface::Chat, MACOS_F19_KEY_CODE),
            None
        );
        assert_eq!(
            claude_destination(ClaudeSurface::Chat, MACOS_F20_KEY_CODE),
            None
        );
    }

    #[test]
    fn claude_code_uses_only_documented_desktop_shortcuts() {
        assert_eq!(
            claude_destination(ClaudeSurface::Code, MACOS_F16_KEY_CODE),
            Some((MACOS_ESCAPE_KEY_CODE, CGEventFlags::empty()))
        );
        assert_eq!(
            claude_destination(ClaudeSurface::Code, MACOS_F17_KEY_CODE),
            Some((MACOS_RETURN_KEY_CODE, CGEventFlags::empty()))
        );
        assert_eq!(
            claude_destination(ClaudeSurface::Code, MACOS_F18_KEY_CODE),
            Some((
                MACOS_E_KEY_CODE,
                CGEventFlags::CGEventFlagCommand | CGEventFlags::CGEventFlagShift
            ))
        );
        assert_eq!(
            claude_destination(ClaudeSurface::Code, MACOS_F19_KEY_CODE),
            Some((
                MACOS_E_KEY_CODE,
                CGEventFlags::CGEventFlagCommand | CGEventFlags::CGEventFlagShift
            ))
        );
        assert_eq!(
            claude_destination(ClaudeSurface::Code, MACOS_F20_KEY_CODE),
            Some((
                MACOS_I_KEY_CODE,
                CGEventFlags::CGEventFlagCommand | CGEventFlags::CGEventFlagShift
            ))
        );
    }

    #[test]
    fn unknown_claude_surface_never_receives_surface_specific_shortcuts() {
        assert_eq!(
            claude_destination(ClaudeSurface::Unknown, MACOS_F18_KEY_CODE),
            None
        );
        assert_eq!(
            claude_destination(ClaudeSurface::Unknown, MACOS_F19_KEY_CODE),
            None
        );
        assert_eq!(
            claude_destination(ClaudeSurface::Unknown, MACOS_F20_KEY_CODE),
            None
        );
    }
}
