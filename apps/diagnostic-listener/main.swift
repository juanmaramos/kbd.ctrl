import AppKit
import Foundation
import IOKit.hid

private let targetVendorID = 0x514c
private let targetProductID = 0x8850
private let discoverMode = CommandLine.arguments.contains("--discover")
private let controllerKeys = Set([UInt8(0x68)] + Array(UInt8(0x6b)...UInt8(0x6f)))

private final class DeviceContext {
    let device: IOHIDDevice
    let buffer: UnsafeMutablePointer<UInt8>
    var previousKeyboardUsages = Set<UInt8>()
    var previousButtons: UInt8 = 0
    var previousConsumerUsage: UInt16 = 0

    init(device: IOHIDDevice, reportSize: Int) {
        self.device = device
        buffer = .allocate(capacity: reportSize)
        buffer.initialize(repeating: 0, count: reportSize)
    }

    deinit {
        buffer.deallocate()
    }

    func handle(reportID: UInt32, report: UnsafeMutablePointer<UInt8>, length: Int) {
        guard length > 0 else { return }

        var bytes = Array(UnsafeBufferPointer(start: report, count: length))
        if bytes.first == UInt8(truncatingIfNeeded: reportID) {
            bytes.removeFirst()
        }

        switch reportID {
        case 1, 4:
            handleKeyboardReport(bytes)
        case 2:
            handlePointerReport(bytes)
        case 5:
            handleConsumerReport(bytes)
        default:
            break
        }
    }

    private func handleKeyboardReport(_ bytes: [UInt8]) {
        guard bytes.count >= 3 else { return }

        let modifiers = bytes[0]
        let current = Set(bytes.dropFirst(2).filter { $0 != 0 })
        let pressed = current.subtracting(previousKeyboardUsages)
        let released = previousKeyboardUsages.subtracting(current)

        for usage in pressed.sorted() where discoverMode || controllerKeys.contains(usage) {
            log(
                event: keyboardLabel(usage),
                state: "down",
                usagePage: 0x07,
                usage: Int(usage),
                modifiers: modifiers
            )
        }

        for usage in released.sorted() where discoverMode || controllerKeys.contains(usage) {
            log(
                event: keyboardLabel(usage),
                state: "up",
                usagePage: 0x07,
                usage: Int(usage),
                modifiers: modifiers
            )
        }

        previousKeyboardUsages = current
    }

    private func handlePointerReport(_ bytes: [UInt8]) {
        guard bytes.count >= 4 else { return }

        let buttons = bytes[0] & 0x07
        let changedButtons = previousButtons ^ buttons

        for index in 0..<3 {
            let mask = UInt8(1 << index)
            guard changedButtons & mask != 0 else { continue }
            log(
                event: "pointer_button_\(index + 1)",
                state: buttons & mask == 0 ? "up" : "down",
                usagePage: 0x09,
                usage: index + 1,
                modifiers: 0
            )
        }

        let wheel = Int(Int8(bitPattern: bytes[3]))
        if wheel != 0 {
            log(
                event: wheel > 0 ? "wheel_up" : "wheel_down",
                state: "step",
                usagePage: 0x01,
                usage: 0x38,
                modifiers: 0,
                value: wheel
            )
        }

        previousButtons = buttons
    }

    private func handleConsumerReport(_ bytes: [UInt8]) {
        guard bytes.count >= 2 else { return }

        let current = UInt16(bytes[0]) | (UInt16(bytes[1]) << 8)

        if previousConsumerUsage != 0, current != previousConsumerUsage {
            log(
                event: consumerLabel(previousConsumerUsage),
                state: "up",
                usagePage: 0x0c,
                usage: Int(previousConsumerUsage),
                modifiers: 0
            )
        }

        if current != 0, current != previousConsumerUsage {
            log(
                event: consumerLabel(current),
                state: "down",
                usagePage: 0x0c,
                usage: Int(current),
                modifiers: 0
            )
        }

        previousConsumerUsage = current
    }

    private func log(
        event: String,
        state: String,
        usagePage: Int,
        usage: Int,
        modifiers: UInt8,
        value: Int? = nil
    ) {
        let application = NSWorkspace.shared.frontmostApplication
        var record: [String: Any] = [
            "timestamp": ISO8601DateFormatter().string(from: Date()),
            "device": propertyString(device, key: kIOHIDProductKey) ?? "unknown",
            "vendor_id": String(format: "0x%04x", targetVendorID),
            "product_id": String(format: "0x%04x", targetProductID),
            "event": event,
            "state": state,
            "usage_page": String(format: "0x%02x", usagePage),
            "usage": String(format: "0x%02x", usage),
            "modifiers": modifierNames(modifiers),
            "foreground_application":
                application?.bundleIdentifier ?? application?.localizedName ?? "unknown",
        ]

        if let value {
            record["value"] = value
        }

        guard
            let data = try? JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]),
            let line = String(data: data, encoding: .utf8)
        else {
            return
        }

        print(line)
        fflush(stdout)
    }
}

private var contexts = [DeviceContext]()

private func propertyNumber(_ device: IOHIDDevice, key: String) -> Int? {
    (IOHIDDeviceGetProperty(device, key as CFString) as? NSNumber)?.intValue
}

private func propertyString(_ device: IOHIDDevice, key: String) -> String? {
    IOHIDDeviceGetProperty(device, key as CFString) as? String
}

private func keyboardLabel(_ usage: UInt8) -> String {
    if controllerKeys.contains(usage) {
        return "F\(Int(usage) - 0x68 + 13)"
    }
    return String(format: "keyboard_usage_0x%02x", usage)
}

private func consumerLabel(_ usage: UInt16) -> String {
    switch usage {
    case 0x00e2: return "mute"
    case 0x00e9: return "volume_up"
    case 0x00ea: return "volume_down"
    case 0x00b5: return "next_track"
    case 0x00b6: return "previous_track"
    case 0x00cd: return "play_pause"
    default: return String(format: "consumer_usage_0x%04x", usage)
    }
}

private func modifierNames(_ byte: UInt8) -> [String] {
    let names = [
        "left_control",
        "left_shift",
        "left_option",
        "left_command",
        "right_control",
        "right_shift",
        "right_option",
        "right_command",
    ]

    return names.enumerated().compactMap { index, name in
        byte & UInt8(1 << index) == 0 ? nil : name
    }
}

private let reportCallback: IOHIDReportCallback = {
    context, result, _, _, reportID, report, reportLength in

    guard
        result == kIOReturnSuccess,
        let context
    else {
        return
    }

    let deviceContext = Unmanaged<DeviceContext>
        .fromOpaque(context)
        .takeUnretainedValue()

    deviceContext.handle(
        reportID: reportID,
        report: report,
        length: reportLength
    )
}

private let deviceMatchedCallback: IOHIDDeviceCallback = {
    _, result, _, device in

    guard result == kIOReturnSuccess else { return }

    let usagePage = propertyNumber(device, key: kIOHIDPrimaryUsagePageKey)
    let usage = propertyNumber(device, key: kIOHIDPrimaryUsageKey)

    // The other collection is the vendor-defined configuration channel.
    guard usagePage == 0x01, usage == 0x06 else { return }

    let reportSize = max(
        propertyNumber(device, key: kIOHIDMaxInputReportSizeKey) ?? 64,
        64
    )
    let context = DeviceContext(device: device, reportSize: reportSize)
    contexts.append(context)

    IOHIDDeviceRegisterInputReportCallback(
        device,
        context.buffer,
        reportSize,
        reportCallback,
        Unmanaged.passUnretained(context).toOpaque()
    )

    let product = propertyString(device, key: kIOHIDProductKey) ?? "unknown"
    let serial = propertyString(device, key: kIOHIDSerialNumberKey) ?? "unknown"
    fputs(
        "Connected: \(product), serial \(serial). " +
        (discoverMode
            ? "Discovery mode logs numeric usages from this device only.\n"
            : "Restricted mode logs F13 and F16-F20, wheel, pointer buttons, and consumer controls only.\n"),
        stderr
    )
}

let manager = IOHIDManagerCreate(
    kCFAllocatorDefault,
    IOOptionBits(kIOHIDOptionsTypeNone)
)

let matching: [String: Any] = [
    kIOHIDVendorIDKey as String: targetVendorID,
    kIOHIDProductIDKey as String: targetProductID,
]

IOHIDManagerSetDeviceMatching(manager, matching as CFDictionary)
IOHIDManagerRegisterDeviceMatchingCallback(
    manager,
    deviceMatchedCallback,
    nil
)
IOHIDManagerScheduleWithRunLoop(
    manager,
    CFRunLoopGetCurrent(),
    CFRunLoopMode.defaultMode.rawValue
)

let openResult = IOHIDManagerOpen(
    manager,
    IOOptionBits(kIOHIDOptionsTypeNone)
)

guard openResult == kIOReturnSuccess else {
    fputs(
        "Could not open HID manager (IOReturn \(openResult)). " +
        "Grant the app launching this listener Input Monitoring permission in System Settings > " +
        "Privacy & Security, then retry.\n",
        stderr
    )
    exit(1)
}

fputs(
    "Waiting for 0x514c:0x8850. Press Control-C to stop.\n",
    stderr
)
CFRunLoopRun()
