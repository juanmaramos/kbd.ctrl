import {
  defaultRoleColors,
  type DeviceBackupSummary,
  type HardwareInputEvent,
  type MappingStatus,
  type MappingSummary,
  type RgbApplySummary,
} from "@/lib/device"

export const controllerControlCodes = [
  "F13",
  "F16",
  "F17",
  "F18",
  "F19",
  "F20",
] as const

export type HardwareSetupPhase = "backup" | "mapping" | "lighting"

type HardwareSetupDependencies = {
  backup: () => Promise<DeviceBackupSummary>
  configureMapping: (backupPath: string) => Promise<MappingSummary>
  applyLighting: (
    profile: "reactive",
    colors: string[]
  ) => Promise<RgbApplySummary>
  onBackup?: (backup: DeviceBackupSummary) => void
  onMapping?: (mapping: MappingSummary) => void
  onPhase: (phase: HardwareSetupPhase) => void
}

export async function applyDefaultLighting(
  applyLighting: HardwareSetupDependencies["applyLighting"]
) {
  return applyLighting("reactive", [...defaultRoleColors])
}

export async function initializeExistingHardware(
  inspectMapping: () => Promise<MappingStatus>,
  applyLighting: HardwareSetupDependencies["applyLighting"]
) {
  const mappingStatus = await inspectMapping()
  if (!mappingStatus.configured) {
    return false
  }

  await applyDefaultLighting(applyLighting)
  return true
}

export async function configureHardware({
  backup,
  configureMapping,
  applyLighting,
  onBackup,
  onMapping,
  onPhase,
}: HardwareSetupDependencies) {
  onPhase("backup")
  const backupSummary = await backup()
  onBackup?.(backupSummary)

  onPhase("mapping")
  const mappingSummary = await configureMapping(backupSummary.path)
  onMapping?.(mappingSummary)

  onPhase("lighting")
  const lightingSummary = await applyDefaultLighting(applyLighting)

  return {
    backup: backupSummary,
    mapping: mappingSummary,
    lighting: lightingSummary,
  }
}

export function recordTestedControl(
  current: Set<string>,
  event: Pick<HardwareInputEvent, "control" | "state">
): Set<string> {
  if (
    !controllerControlCodes.includes(
      event.control as (typeof controllerControlCodes)[number]
    ) ||
    (event.state !== "down" && event.state !== "step")
  ) {
    return current
  }

  const next = new Set(current)
  next.add(event.control)
  return next
}
