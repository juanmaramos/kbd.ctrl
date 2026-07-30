import CoreFoundation
import Foundation
import IOKit.hid

private let vendorID = 0x514c
private let productID = 0x8850
private let vendorUsagePage = 0xff00
private let vendorUsage = 0x01
private let reportID: CFIndex = 3

private final class ResponseCapture {
    var response: [UInt8]?
    let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: 65)

    init() {
        buffer.initialize(repeating: 0, count: 65)
    }

    deinit {
        buffer.deallocate()
    }
}

private let reportCallback: IOHIDReportCallback = {
    context, result, _, _, incomingReportID, report, reportLength in

    guard
        result == kIOReturnSuccess,
        incomingReportID == UInt32(reportID),
        let context
    else {
        return
    }

    let capture = Unmanaged<ResponseCapture>
        .fromOpaque(context)
        .takeUnretainedValue()
    capture.response = Array(
        UnsafeBufferPointer(start: report, count: reportLength)
    )
    CFRunLoopStop(CFRunLoopGetCurrent())
}

private func hex(_ bytes: [UInt8]) -> String {
    bytes.map { String(format: "%02x", $0) }.joined(separator: " ")
}

let manager = IOHIDManagerCreate(
    kCFAllocatorDefault,
    IOOptionBits(kIOHIDOptionsTypeNone)
)
let matching: [String: Any] = [
    kIOHIDVendorIDKey as String: vendorID,
    kIOHIDProductIDKey as String: productID,
    kIOHIDPrimaryUsagePageKey as String: vendorUsagePage,
    kIOHIDPrimaryUsageKey as String: vendorUsage,
]
IOHIDManagerSetDeviceMatching(manager, matching as CFDictionary)

let managerResult = IOHIDManagerOpen(
    manager,
    IOOptionBits(kIOHIDOptionsTypeNone)
)
guard managerResult == kIOReturnSuccess else {
    fputs("Could not open vendor HID manager: \(managerResult)\n", stderr)
    exit(1)
}

guard
    let devices = IOHIDManagerCopyDevices(manager) as? Set<IOHIDDevice>,
    let device = devices.first
else {
    fputs("Vendor configuration interface 0x514c:0x8850 was not found.\n", stderr)
    exit(1)
}

let deviceResult = IOHIDDeviceOpen(
    device,
    IOOptionBits(kIOHIDOptionsTypeNone)
)
guard deviceResult == kIOReturnSuccess else {
    fputs("Could not open vendor configuration interface: \(deviceResult)\n", stderr)
    exit(1)
}
defer {
    IOHIDDeviceClose(device, IOOptionBits(kIOHIDOptionsTypeNone))
}

private let capture = ResponseCapture()
IOHIDDeviceRegisterInputReportCallback(
    device,
    capture.buffer,
    65,
    reportCallback,
    Unmanaged.passUnretained(capture).toOpaque()
)
IOHIDDeviceScheduleWithRunLoop(
    device,
    CFRunLoopGetCurrent(),
    CFRunLoopMode.defaultMode.rawValue
)
defer {
    IOHIDDeviceUnscheduleFromRunLoop(
        device,
        CFRunLoopGetCurrent(),
        CFRunLoopMode.defaultMode.rawValue
    )
}

// Read-only device-information request recovered from the vendor configurator:
// HIDAPI packet 03 fb fb fb followed by zero padding. IOHID receives the report
// ID separately, so this is the 64-byte payload after byte 03.
var request = [UInt8](repeating: 0, count: 64)
request[0] = 0xfb
request[1] = 0xfb
request[2] = 0xfb

let writeResult = request.withUnsafeBytes { bytes in
    IOHIDDeviceSetReport(
        device,
        kIOHIDReportTypeOutput,
        reportID,
        bytes.bindMemory(to: UInt8.self).baseAddress!,
        request.count
    )
}
guard writeResult == kIOReturnSuccess else {
    fputs("The read-only information request failed: \(writeResult)\n", stderr)
    exit(1)
}

CFRunLoopRunInMode(.defaultMode, 0.5, false)

guard let response = capture.response else {
    fputs("The device did not answer the information request.\n", stderr)
    exit(1)
}

print("response=\(hex(response))")
if response.count >= 5 {
    print(
        "reported_key_count=\(response[2]) " +
        "reported_led_count=\(response[3]) " +
        "reported_aux_count=\(response[4])"
    )
}
