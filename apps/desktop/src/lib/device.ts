import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

export type DeviceStatus = {
  connected: boolean
  product: string | null
  serialNumber: string | null
  vendorId: string
  productId: string
  configurationInterfaceVisible: boolean
  keyboardInterfaceVisible: boolean
  inputMonitoringGranted: boolean
  controlAccessGranted: boolean
  error: string | null
}

export type HardwareInputEvent = {
  sequence: number
  timestampMs: number
  control: string
  state: "down" | "up" | "step" | "report"
  usagePage: string
  usage: string
  value: number | null
  durationMs: number | null
  modifiers: string[]
  reportId: number | null
  raw: string
}

export type InputMonitorStatus = {
  state: "starting" | "listening" | "warning" | "error" | "stopped"
  interfaceCount: number
  message: string
}

export type FrontmostApplication = {
  name: string | null
  bundleId: string | null
}

export type DeviceBackupSummary = {
  path: string
  fingerprintSha256: string
  reportCount: number
  capturedAtMs: number
  deviceInfo: number[]
}

export type MappingSummary = {
  prewriteBackupPath: string
  verificationFingerprintSha256: string
  writtenReportCount: number
  mappings: string[]
}

export type MappingStatus = {
  configured: boolean
  message: string
}

export type AppPreferences = {
  showDockIcon: boolean
  launchAtLogin: boolean
  onboarding: OnboardingState
}

export type OnboardingState = {
  completed: boolean
  dismissed: boolean
  hardwareConfigured: boolean
  codexConfigured: boolean
}

export type RgbStatus = {
  available: boolean
  profile: "reactive" | "static" | "custom" | null
  modes: number[]
  roleColors: string[]
  message: string
}

export type RgbApplySummary = {
  profile: "reactive" | "static"
  backupPath: string
  verifiedLayers: number
  roleColors: string[]
}

export const defaultRoleColors = ["#874EFE", "#FF6251", "#96D35F"]

export const previewDeviceStatus: DeviceStatus = {
  connected: false,
  product: null,
  serialNumber: null,
  vendorId: "0x514c",
  productId: "0x8850",
  configurationInterfaceVisible: false,
  keyboardInterfaceVisible: false,
  inputMonitoringGranted: false,
  controlAccessGranted: false,
  error: null,
}

export async function readDeviceStatus(): Promise<DeviceStatus> {
  if (!("__TAURI_INTERNALS__" in window)) {
    return previewDeviceStatus
  }

  return invoke<DeviceStatus>("get_device_status")
}

export async function readFrontmostApplication(): Promise<FrontmostApplication> {
  if (!("__TAURI_INTERNALS__" in window)) {
    return { name: null, bundleId: null }
  }

  return invoke<FrontmostApplication>("get_frontmost_application")
}

export async function requestInputMonitoringAccess(): Promise<boolean> {
  if (!("__TAURI_INTERNALS__" in window)) {
    return false
  }

  return invoke<boolean>("request_input_monitoring_access")
}

export async function requestControlAccess(): Promise<boolean> {
  if (!("__TAURI_INTERNALS__" in window)) {
    return false
  }

  return invoke<boolean>("request_control_access")
}

export async function readAppPreferences(): Promise<AppPreferences> {
  if (!("__TAURI_INTERNALS__" in window)) {
    return {
      showDockIcon: false,
      launchAtLogin: false,
      onboarding: {
        completed: false,
        dismissed: false,
        hardwareConfigured: false,
        codexConfigured: false,
      },
    }
  }

  return invoke<AppPreferences>("get_app_preferences")
}

export async function updateOnboardingState(
  onboarding: OnboardingState
): Promise<AppPreferences> {
  if (!("__TAURI_INTERNALS__" in window)) {
    return {
      showDockIcon: false,
      launchAtLogin: false,
      onboarding,
    }
  }

  return invoke<AppPreferences>("set_onboarding_state", { onboarding })
}

export async function openCodex(): Promise<boolean> {
  if (!("__TAURI_INTERNALS__" in window)) {
    return false
  }

  return invoke<boolean>("open_codex")
}

export async function updateDockVisibility(
  visible: boolean
): Promise<AppPreferences> {
  return invoke<AppPreferences>("set_show_dock_icon", { visible })
}

export async function updateLaunchAtLogin(
  enabled: boolean
): Promise<AppPreferences> {
  return invoke<AppPreferences>("set_launch_at_login", { enabled })
}

export async function readRgbStatus(): Promise<RgbStatus> {
  if (!("__TAURI_INTERNALS__" in window)) {
    return {
      available: false,
      profile: null,
      modes: [],
      roleColors: defaultRoleColors,
      message: "Connect by USB to inspect lighting.",
    }
  }

  return invoke<RgbStatus>("get_rgb_status")
}

export async function applyRgbProfile(
  profile: "reactive" | "static",
  roleColors: string[]
): Promise<RgbApplySummary> {
  return invoke<RgbApplySummary>("apply_rgb_profile", {
    profile,
    roleColors,
  })
}

export async function backupDeviceConfiguration(): Promise<DeviceBackupSummary> {
  return invoke<DeviceBackupSummary>("backup_device_configuration")
}

export async function configureTransportMapping(
  backupPath: string
): Promise<MappingSummary> {
  return invoke<MappingSummary>("configure_transport_mapping", { backupPath })
}

export async function inspectTransportMapping(): Promise<MappingStatus> {
  return invoke<MappingStatus>("inspect_transport_mapping")
}

export async function startInputMonitor(): Promise<void> {
  return invoke("start_input_monitor")
}

export async function stopInputMonitor(): Promise<void> {
  return invoke("stop_input_monitor")
}

export async function testCodexTransport(
  control: "F18" | "F19" | "F20"
): Promise<void> {
  return invoke("test_codex_transport", { control })
}

export async function listenToInputEvents(
  handler: (event: HardwareInputEvent) => void
): Promise<UnlistenFn> {
  return listen<HardwareInputEvent>("kbd-input", ({ payload }) => {
    handler(payload)
  })
}

export async function listenToInputMonitorStatus(
  handler: (status: InputMonitorStatus) => void
): Promise<UnlistenFn> {
  return listen<InputMonitorStatus>("kbd-monitor-status", ({ payload }) => {
    handler(payload)
  })
}
