import { describe, expect, it, vi } from "vitest"

import { defaultRoleColors } from "@/lib/device"
import {
  applyDefaultLighting,
  configureHardware,
  initializeExistingHardware,
  recordTestedControl,
} from "@/lib/hardware-setup"

const backup = {
  path: "/tmp/keypad-backup.json",
  fingerprintSha256: "backup-fingerprint",
  reportCount: 72,
  capturedAtMs: 1,
  deviceInfo: [3, 1, 3, 1, 11],
}

const mapping = {
  prewriteBackupPath: backup.path,
  verificationFingerprintSha256: "mapping-fingerprint",
  writtenReportCount: 19,
  mappings: ["left key → F13"],
}

const lighting = {
  profile: "reactive" as const,
  backupPath: "/tmp/rgb-backup.json",
  verifiedLayers: 3,
  roleColors: defaultRoleColors,
}

describe("configureHardware", () => {
  it("backs up, maps, and verifies the default lighting in order", async () => {
    const calls: string[] = []
    const result = await configureHardware({
      backup: vi.fn(async () => {
        calls.push("backup")
        return backup
      }),
      configureMapping: vi.fn(async (path) => {
        expect(path).toBe(backup.path)
        calls.push("mapping")
        return mapping
      }),
      applyLighting: vi.fn(async (profile, colors) => {
        expect(profile).toBe("reactive")
        expect(colors).toEqual(defaultRoleColors)
        calls.push("lighting")
        return lighting
      }),
      onPhase: (phase) => calls.push(`phase:${phase}`),
    })

    expect(calls).toEqual([
      "phase:backup",
      "backup",
      "phase:mapping",
      "mapping",
      "phase:lighting",
      "lighting",
    ])
    expect(result).toEqual({ backup, mapping, lighting })
  })

  it("does not report success when default lighting fails", async () => {
    await expect(
      configureHardware({
        backup: async () => backup,
        configureMapping: async () => mapping,
        applyLighting: async () => {
          throw new Error("RGB verification failed")
        },
        onPhase: () => undefined,
      })
    ).rejects.toThrow("RGB verification failed")
  })
})

describe("applyDefaultLighting", () => {
  it("uses the published purple, red, and green defaults", async () => {
    const applyLighting = vi.fn(async () => lighting)

    await applyDefaultLighting(applyLighting)

    expect(applyLighting).toHaveBeenCalledWith("reactive", [
      "#874EFE",
      "#FF6251",
      "#96D35F",
    ])
  })
})

describe("initializeExistingHardware", () => {
  it("applies defaults before accepting an existing mapping", async () => {
    const applyLighting = vi.fn(async () => lighting)

    await expect(
      initializeExistingHardware(
        async () => ({ configured: true, message: "Mapped" }),
        applyLighting
      )
    ).resolves.toBe(true)
    expect(applyLighting).toHaveBeenCalledWith("reactive", defaultRoleColors)
  })

  it("does not write lighting when the mapping still needs configuration", async () => {
    const applyLighting = vi.fn(async () => lighting)

    await expect(
      initializeExistingHardware(
        async () => ({ configured: false, message: "Not mapped" }),
        applyLighting
      )
    ).resolves.toBe(false)
    expect(applyLighting).not.toHaveBeenCalled()
  })
})

describe("recordTestedControl", () => {
  it("requires all six unique controller actions", () => {
    let tested = new Set<string>()

    for (const control of ["F13", "F16", "F17", "F18", "F19", "F20"]) {
      tested = recordTestedControl(tested, { control, state: "down" })
    }

    expect(tested.size).toBe(6)
  })

  it("ignores duplicates, releases, and unrelated keys", () => {
    const initial = new Set(["F13"])
    const duplicate = recordTestedControl(initial, {
      control: "F13",
      state: "down",
    })
    const release = recordTestedControl(duplicate, {
      control: "F16",
      state: "up",
    })
    const unrelated = recordTestedControl(release, {
      control: "Return",
      state: "down",
    })

    expect(duplicate.size).toBe(1)
    expect(release.size).toBe(1)
    expect(unrelated.size).toBe(1)
  })
})
