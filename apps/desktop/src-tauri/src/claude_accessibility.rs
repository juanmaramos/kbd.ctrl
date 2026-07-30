use std::{ffi::c_void, thread, time::Duration};

use accessibility::{
    AXAttribute, AXUIElement, AXUIElementActions, AXUIElementAttributes, ElementFinder,
};
use accessibility_sys::{
    kAXPositionAttribute, kAXSizeAttribute, kAXURLAttribute, kAXValueTypeCGPoint,
    kAXValueTypeCGSize, AXValueGetValue, AXValueRef,
};
use core_foundation::{
    base::{CFType, TCFType},
    string::CFString,
    url::CFURL,
};
use core_graphics::{
    event::{CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGMouseButton},
    event_source::{CGEventSource, CGEventSourceStateID},
    geometry::CGPoint,
};

pub const CLAUDE_BUNDLE_ID: &str = "com.anthropic.claudefordesktop";

const MACOS_RETURN_KEY_CODE: i64 = 0x24;
const MACOS_RIGHT_KEY_CODE: i64 = 0x7c;
const MACOS_DOWN_KEY_CODE: i64 = 0x7d;
const MACOS_UP_KEY_CODE: i64 = 0x7e;
const MACOS_HOME_KEY_CODE: i64 = 0x73;
const MACOS_END_KEY_CODE: i64 = 0x77;

const EFFORT_LEVELS: [&str; 5] = ["Low", "Medium", "High", "Extra", "Max"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaudeSurface {
    Chat,
    Code,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffortDirection {
    Decrease,
    Increase,
}

#[derive(Clone, Copy, Debug)]
pub struct VoiceTarget {
    pub center: CGPoint,
}

fn claude_application() -> Result<AXUIElement, String> {
    AXUIElement::application_with_bundle(CLAUDE_BUNDLE_ID).map_err(|error| error.to_string())
}

fn element_text(element: &AXUIElement) -> Vec<String> {
    [
        element.title().ok(),
        element.description().ok(),
        element.value_description().ok(),
    ]
    .into_iter()
    .flatten()
    .map(|value| value.to_string())
    .filter(|value| !value.is_empty())
    .collect()
}

fn find_element(
    application: &AXUIElement,
    predicate: impl 'static + Fn(&AXUIElement) -> bool,
) -> Result<AXUIElement, String> {
    ElementFinder::new(application, predicate, None)
        .find()
        .map_err(|error| error.to_string())
}

fn find_element_with_text(
    application: &AXUIElement,
    predicate: impl 'static + Fn(&str) -> bool,
) -> Result<AXUIElement, String> {
    find_element(application, move |element| {
        element_text(element)
            .iter()
            .any(|text| predicate(text.as_str()))
    })
}

pub fn surface() -> ClaudeSurface {
    let Ok(application) = claude_application() else {
        return ClaudeSurface::Unknown;
    };

    let chat_voice_control = find_element_with_text(&application, |text| {
        text == "Press and hold to record" || text == "Use voice mode"
    });

    if chat_voice_control.is_ok() {
        ClaudeSurface::Chat
    } else if find_element(&application, |element| {
        element_url(element).is_some_and(|url| url.contains("claude.ai/code"))
    })
    .is_ok()
    {
        ClaudeSurface::Code
    } else {
        ClaudeSurface::Unknown
    }
}

fn element_url(element: &AXUIElement) -> Option<String> {
    ax_attribute(element, kAXURLAttribute)
        .ok()?
        .downcast::<CFURL>()
        .map(|url| url.get_string().to_string())
}

fn model_picker(application: &AXUIElement) -> Result<AXUIElement, String> {
    find_element_with_text(application, |text| text.starts_with("Model: "))
}

fn model_picker_label(element: &AXUIElement) -> Option<String> {
    element_text(element)
        .into_iter()
        .find(|text| text.starts_with("Model: "))
}

pub fn open_chat_model_picker() -> Result<(), String> {
    ensure_claude_frontmost()?;
    let application = claude_application()?;
    model_picker(&application)?
        .press()
        .map_err(|error| error.to_string())
}

fn effort_index(label: &str) -> Option<usize> {
    EFFORT_LEVELS
        .iter()
        .position(|effort| label.split_whitespace().last() == Some(*effort))
}

fn destination_effort(current: usize, direction: EffortDirection) -> usize {
    match direction {
        EffortDirection::Decrease => current.saturating_sub(1),
        EffortDirection::Increase => (current + 1).min(EFFORT_LEVELS.len() - 1),
    }
}

pub fn adjust_chat_effort(direction: EffortDirection) -> Result<(), String> {
    ensure_claude_frontmost()?;
    let application = claude_application()?;
    let picker = model_picker(&application)?;
    let label = model_picker_label(&picker)
        .ok_or_else(|| "Claude's model selector is unavailable.".to_owned())?;
    let current = effort_index(&label)
        .ok_or_else(|| format!("Claude's current effort could not be read from “{label}”."))?;
    let destination = destination_effort(current, direction);

    picker.press().map_err(|error| error.to_string())?;
    thread::sleep(Duration::from_millis(70));
    ensure_claude_frontmost()?;

    // Claude Chat opens the model menu on its first item. End selects
    // “More models”, Up selects “Effort”, and Right opens the effort submenu.
    for key_code in [MACOS_END_KEY_CODE, MACOS_UP_KEY_CODE, MACOS_RIGHT_KEY_CODE] {
        ensure_claude_frontmost()?;
        post_key(key_code, CGEventFlags::empty())?;
        thread::sleep(Duration::from_millis(35));
    }

    // The effort submenu opens on Low. Home makes that starting point explicit,
    // then Down steps to the effort derived from Claude's own current label.
    post_key(MACOS_HOME_KEY_CODE, CGEventFlags::empty())?;
    for _ in 0..destination {
        ensure_claude_frontmost()?;
        post_key(MACOS_DOWN_KEY_CODE, CGEventFlags::empty())?;
    }
    ensure_claude_frontmost()?;
    post_key(MACOS_RETURN_KEY_CODE, CGEventFlags::empty())
}

fn ensure_claude_frontmost() -> Result<(), String> {
    (crate::app_context::frontmost_bundle_id().as_deref() == Some(CLAUDE_BUNDLE_ID))
        .then_some(())
        .ok_or_else(|| "Claude is no longer the active app.".to_owned())
}

pub fn voice_target() -> Result<VoiceTarget, String> {
    let application = claude_application()?;
    let button = find_element_with_text(&application, |text| text == "Press and hold to record")?;
    let position = ax_point(&button, kAXPositionAttribute, kAXValueTypeCGPoint)?;
    let size = ax_size(&button)?;

    Ok(VoiceTarget {
        center: CGPoint::new(
            position.x + size.width / 2.0,
            position.y + size.height / 2.0,
        ),
    })
}

pub fn post_voice_mouse_event(target: VoiceTarget, is_down: bool) -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "Could not create a mouse event source.".to_owned())?;
    let event = CGEvent::new_mouse_event(
        source,
        if is_down {
            CGEventType::LeftMouseDown
        } else {
            CGEventType::LeftMouseUp
        },
        target.center,
        CGMouseButton::Left,
    )
    .map_err(|_| "Could not create Claude's voice mouse event.".to_owned())?;
    event.post(CGEventTapLocation::HID);
    Ok(())
}

pub fn current_pointer_location() -> Result<CGPoint, String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "Could not create a pointer event source.".to_owned())?;
    CGEvent::new(source)
        .map(|event| event.location())
        .map_err(|_| "Could not read the pointer location.".to_owned())
}

pub fn restore_pointer(location: CGPoint) -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "Could not create a pointer event source.".to_owned())?;
    let event = CGEvent::new_mouse_event(
        source,
        CGEventType::MouseMoved,
        location,
        CGMouseButton::Left,
    )
    .map_err(|_| "Could not restore the pointer location.".to_owned())?;
    event.post(CGEventTapLocation::HID);
    Ok(())
}

fn post_key(key_code: i64, flags: CGEventFlags) -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "Could not create a virtual keyboard event source.".to_owned())?;
    let key_code = u16::try_from(key_code)
        .map_err(|_| "The virtual key code is outside the macOS range.".to_owned())?;
    let down = CGEvent::new_keyboard_event(source.clone(), key_code, true)
        .map_err(|_| "Could not create the virtual key-down event.".to_owned())?;
    let up = CGEvent::new_keyboard_event(source, key_code, false)
        .map_err(|_| "Could not create the virtual key-up event.".to_owned())?;
    down.set_flags(flags);
    up.set_flags(flags);
    down.post(CGEventTapLocation::HID);
    thread::sleep(Duration::from_millis(28));
    up.post(CGEventTapLocation::HID);
    Ok(())
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct AxPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct AxSize {
    width: f64,
    height: f64,
}

fn ax_attribute(element: &AXUIElement, name: &str) -> Result<CFType, String> {
    let attribute = AXAttribute::<CFType>::new(&CFString::new(name));
    element
        .attribute(&attribute)
        .map_err(|error| error.to_string())
}

fn ax_point(element: &AXUIElement, name: &str, value_type: u32) -> Result<AxPoint, String> {
    let value = ax_attribute(element, name)?;
    let mut point = AxPoint::default();
    let copied = unsafe {
        AXValueGetValue(
            value.as_CFTypeRef() as AXValueRef,
            value_type,
            (&mut point as *mut AxPoint).cast::<c_void>(),
        )
    };

    copied
        .then_some(point)
        .ok_or_else(|| format!("Claude's {name} accessibility value is unavailable."))
}

fn ax_size(element: &AXUIElement) -> Result<AxSize, String> {
    let value = ax_attribute(element, kAXSizeAttribute)?;
    let mut size = AxSize::default();
    let copied = unsafe {
        AXValueGetValue(
            value.as_CFTypeRef() as AXValueRef,
            kAXValueTypeCGSize,
            (&mut size as *mut AxSize).cast::<c_void>(),
        )
    };

    copied
        .then_some(size)
        .ok_or_else(|| "Claude's voice button size is unavailable.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{destination_effort, effort_index, EffortDirection};

    #[test]
    fn reads_effort_from_claude_model_labels() {
        assert_eq!(effort_index("Model: Sonnet 5 Low"), Some(0));
        assert_eq!(effort_index("Model: Sonnet 5 Medium"), Some(1));
        assert_eq!(effort_index("Model: Sonnet 5 High"), Some(2));
        assert_eq!(effort_index("Model: Opus 5 Extra"), Some(3));
        assert_eq!(effort_index("Model: Opus 5 Max"), Some(4));
        assert_eq!(effort_index("Model: Sonnet 5"), None);
    }

    #[test]
    fn effort_steps_clamp_at_claudes_real_bounds() {
        assert_eq!(destination_effort(0, EffortDirection::Decrease), 0);
        assert_eq!(destination_effort(1, EffortDirection::Decrease), 0);
        assert_eq!(destination_effort(1, EffortDirection::Increase), 2);
        assert_eq!(destination_effort(4, EffortDirection::Increase), 4);
    }
}
