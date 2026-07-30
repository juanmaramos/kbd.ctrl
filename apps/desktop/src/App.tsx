import { useCallback, useEffect, useRef, useState, useTransition } from "react"
import {
  IconActivity,
  IconBluetooth,
  IconBrandGithub,
  IconBrandLinkedin,
  IconBrandX,
  IconDatabaseExport,
  IconDeviceFloppy,
  IconRefresh,
  IconShieldCheck,
  IconShieldLock,
  IconUsb,
} from "@tabler/icons-react"
import { openUrl } from "@tauri-apps/plugin-opener"

import { ControllerSettings } from "@/components/controller-settings"
import { DevicePad } from "@/components/device-pad"
import { Onboarding } from "@/components/onboarding"
import { ProviderSetup } from "@/components/provider-setup"
import {
  Alert,
  AlertAction,
  AlertDescription,
  AlertTitle,
} from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Separator } from "@/components/ui/separator"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { TooltipProvider } from "@/components/ui/tooltip"
import {
  backupDeviceConfiguration,
  applyRgbProfile,
  configureTransportMapping,
  defaultRoleColors,
  inspectTransportMapping,
  listenToInputEvents,
  listenToInputMonitorStatus,
  previewDeviceStatus,
  readAppPreferences,
  readDeviceStatus,
  readFrontmostApplication,
  requestControlAccess,
  requestInputMonitoringAccess,
  startInputMonitor,
  stopInputMonitor,
  updateOnboardingState,
  type AppPreferences,
  type DeviceBackupSummary,
  type DeviceStatus,
  type FrontmostApplication,
  type HardwareInputEvent,
  type InputMonitorStatus,
  type MappingSummary,
} from "@/lib/device"

type StatusRowProps = {
  label: string
  value: string
  state?: "ready" | "pending"
}

export type ConfigurationPhase = "backup" | "mapping" | "lighting" | null

const creatorLinks = [
  {
    href: "https://x.com/jmramox",
    label: "Juan M. Ramos on X",
    icon: IconBrandX,
  },
  {
    href: "https://www.linkedin.com/in/jmramos/",
    label: "Juan M. Ramos on LinkedIn",
    icon: IconBrandLinkedin,
  },
  {
    href: "https://github.com/juanmaramos/kbd.ctrl",
    label: "kbd.ctrl on GitHub",
    icon: IconBrandGithub,
  },
] as const

function StatusRow({ label, value, state = "ready" }: StatusRowProps) {
  return (
    <div className="flex items-center justify-between gap-6 py-3">
      <span className="text-xs font-semibold tracking-widest text-muted-foreground uppercase">
        {label}
      </span>
      <span className="flex items-center gap-2 text-sm font-medium">
        <span className="status-dot" data-state={state} aria-hidden />
        {value}
      </span>
    </div>
  )
}

export function App() {
  const [status, setStatus] = useState<DeviceStatus>(previewDeviceStatus)
  const [preferences, setPreferences] = useState<AppPreferences | null>(null)
  const [showOnboarding, setShowOnboarding] = useState(false)
  const [startOnboardingAtBeginning, setStartOnboardingAtBeginning] =
    useState(false)
  const [onboardingRun, setOnboardingRun] = useState(0)
  const [latestInputEvent, setLatestInputEvent] =
    useState<HardwareInputEvent | null>(null)
  const [testedControls, setTestedControls] = useState<Set<string>>(
    () => new Set()
  )
  const [backup, setBackup] = useState<DeviceBackupSummary | null>(null)
  const [backupError, setBackupError] = useState<string | null>(null)
  const [isBackingUp, setIsBackingUp] = useState(false)
  const [mapping, setMapping] = useState<MappingSummary | null>(null)
  const [mappingError, setMappingError] = useState<string | null>(null)
  const [isMapping, setIsMapping] = useState(false)
  const [configurationPhase, setConfigurationPhase] =
    useState<ConfigurationPhase>(null)
  const inspectedMappingDevice = useRef<string | null>(null)
  const [monitorStatus, setMonitorStatus] = useState<InputMonitorStatus>({
    state: "stopped",
    interfaceCount: 0,
    message: "Starting the controller",
  })
  const [frontmostApplication, setFrontmostApplication] =
    useState<FrontmostApplication>({ name: null, bundleId: null })
  const [isPending, startTransition] = useTransition()

  const refreshStatus = useCallback(() => {
    startTransition(async () => {
      try {
        setStatus(await readDeviceStatus())
      } catch (error) {
        setStatus({
          ...previewDeviceStatus,
          error:
            error instanceof Error
              ? error.message
              : "Unable to inspect the USB service connection.",
        })
      }
    })
  }, [])

  const requestInputAccess = useCallback(async () => {
    await requestInputMonitoringAccess()
    refreshStatus()
  }, [refreshStatus])

  const requestShortcutControl = useCallback(async () => {
    await requestControlAccess()
    refreshStatus()
  }, [refreshStatus])

  const createBackup = useCallback(async () => {
    setIsBackingUp(true)
    setBackupError(null)
    try {
      setBackup(await backupDeviceConfiguration())
    } catch (error) {
      setBackupError(
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Unable to read the device configuration."
      )
    } finally {
      setIsBackingUp(false)
    }
  }, [])

  const applyMapping = useCallback(async () => {
    if (!backup) {
      return
    }

    setIsMapping(true)
    setMappingError(null)
    try {
      setMapping(await configureTransportMapping(backup.path))
    } catch (error) {
      setMappingError(
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Unable to verify the target mapping."
      )
    } finally {
      setIsMapping(false)
    }
  }, [backup])

  const configureForOnboarding = useCallback(async () => {
    setIsBackingUp(true)
    setIsMapping(true)
    setConfigurationPhase("backup")
    setBackupError(null)
    setMappingError(null)
    try {
      const nextBackup = await backupDeviceConfiguration()
      setBackup(nextBackup)
      setConfigurationPhase("mapping")
      const nextMapping = await configureTransportMapping(nextBackup.path)
      setMapping(nextMapping)
      setConfigurationPhase("lighting")
      try {
        await applyRgbProfile("reactive", defaultRoleColors)
      } catch (error) {
        console.warn(
          "The controller mapping verified, but Live pulse lighting was not applied.",
          error
        )
      }
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Unable to configure and verify the controller."
      setMappingError(message)
      throw error
    } finally {
      setIsBackingUp(false)
      setIsMapping(false)
      setConfigurationPhase(null)
    }
  }, [])

  const saveOnboardingState = useCallback(
    async (onboarding: AppPreferences["onboarding"]) => {
      const nextPreferences = await updateOnboardingState(onboarding)
      setPreferences(nextPreferences)
    },
    []
  )

  const dismissOnboarding = useCallback(async () => {
    if (!preferences) {
      return
    }
    await saveOnboardingState({
      ...preferences.onboarding,
      dismissed: true,
    })
    setShowOnboarding(false)
  }, [preferences, saveOnboardingState])

  const completeOnboarding = useCallback(async () => {
    if (!preferences) {
      return
    }
    await saveOnboardingState({
      ...preferences.onboarding,
      completed: true,
      dismissed: false,
    })
    setShowOnboarding(false)
  }, [preferences, saveOnboardingState])

  const runOnboarding = useCallback(async () => {
    if (preferences?.onboarding.dismissed) {
      await saveOnboardingState({
        ...preferences.onboarding,
        dismissed: false,
      })
    }
    setTestedControls(new Set())
    setStartOnboardingAtBeginning(true)
    setOnboardingRun((current) => current + 1)
    setShowOnboarding(true)
  }, [preferences, saveOnboardingState])

  const resumeOnboarding = useCallback(async () => {
    if (preferences?.onboarding.dismissed) {
      await saveOnboardingState({
        ...preferences.onboarding,
        dismissed: false,
      })
    }
    setStartOnboardingAtBeginning(false)
    setOnboardingRun((current) => current + 1)
    setShowOnboarding(true)
  }, [preferences, saveOnboardingState])

  useEffect(() => {
    if (isBackingUp || isMapping) {
      return
    }

    refreshStatus()
    const timer = window.setInterval(refreshStatus, 3_000)
    return () => window.clearInterval(timer)
  }, [isBackingUp, isMapping, refreshStatus])

  useEffect(() => {
    let cancelled = false

    void readAppPreferences()
      .then((nextPreferences) => {
        if (!cancelled) {
          setPreferences(nextPreferences)
          setShowOnboarding(
            !nextPreferences.onboarding.completed &&
              !nextPreferences.onboarding.dismissed
          )
        }
      })
      .catch(() => {
        if (!cancelled) {
          setPreferences({
            showDockIcon: false,
            launchAtLogin: false,
            onboarding: {
              completed: false,
              dismissed: false,
              hardwareConfigured: false,
              codexConfigured: false,
            },
          })
          setShowOnboarding(true)
        }
      })

    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    if (!status.configurationInterfaceVisible) {
      inspectedMappingDevice.current = null
      return
    }
    if (
      !preferences ||
      preferences.onboarding.hardwareConfigured ||
      isBackingUp ||
      isMapping
    ) {
      return
    }

    const deviceKey = status.serialNumber ?? "connected-keypad"
    if (inspectedMappingDevice.current === deviceKey) {
      return
    }
    inspectedMappingDevice.current = deviceKey
    let cancelled = false

    void inspectTransportMapping()
      .then((mappingStatus) => {
        if (!cancelled && mappingStatus.configured) {
          return saveOnboardingState({
            ...preferences.onboarding,
            hardwareConfigured: true,
          })
        }
      })
      .catch((error: unknown) => {
        console.warn(
          "The current keypad mapping could not be inspected.",
          error
        )
      })

    return () => {
      cancelled = true
    }
  }, [
    isBackingUp,
    isMapping,
    preferences,
    saveOnboardingState,
    status.configurationInterfaceVisible,
    status.serialNumber,
  ])

  useEffect(() => {
    let cancelled = false

    async function refreshFrontmostApplication() {
      const application = await readFrontmostApplication()
      if (!cancelled) {
        setFrontmostApplication(application)
      }
    }

    void refreshFrontmostApplication()
    const timer = window.setInterval(() => {
      void refreshFrontmostApplication()
    }, 1_000)

    return () => {
      cancelled = true
      window.clearInterval(timer)
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    let unlistenInput: (() => void) | undefined
    let unlistenStatus: (() => void) | undefined
    let releaseTimer: number | undefined

    async function connectMonitor() {
      const listeners = await Promise.all([
        listenToInputEvents((event) => {
          window.clearTimeout(releaseTimer)
          setLatestInputEvent(event)
          if (
            ["F13", "F16", "F17", "F18", "F19", "F20"].includes(
              event.control
            ) &&
            (event.state === "down" || event.state === "step")
          ) {
            setTestedControls((current) => {
              const next = new Set(current)
              next.add(event.control)
              return next
            })
          }

          if (event.state !== "down") {
            releaseTimer = window.setTimeout(() => {
              setLatestInputEvent((current) =>
                current?.sequence === event.sequence ? null : current
              )
            }, 320)
          }
        }),
        listenToInputMonitorStatus(setMonitorStatus),
      ])

      if (cancelled) {
        listeners.forEach((unlisten) => unlisten())
        return
      }

      ;[unlistenInput, unlistenStatus] = listeners
      await startInputMonitor()
    }

    void connectMonitor().catch((error) => {
      setMonitorStatus({
        state: "error",
        interfaceCount: 0,
        message:
          error instanceof Error
            ? error.message
            : "Unable to start input monitoring",
      })
    })

    return () => {
      cancelled = true
      window.clearTimeout(releaseTimer)
      unlistenInput?.()
      unlistenStatus?.()
      void stopInputMonitor()
    }
  }, [status.controlAccessGranted, status.inputMonitoringGranted])

  const controlsActive =
    monitorStatus.state === "listening" &&
    status.inputMonitoringGranted &&
    status.controlAccessGranted
  const transportLabel = status.configurationInterfaceVisible
    ? "USB service connected"
    : "Wireless controls"
  const openExternalLink = useCallback(
    (event: React.MouseEvent<HTMLAnchorElement>) => {
      event.preventDefault()
      void openUrl(event.currentTarget.href)
    },
    []
  )

  return (
    <TooltipProvider>
      <main className="flex min-h-svh flex-col bg-background">
        <header className="app-header">
          <div className="flex min-w-0 items-center gap-4">
            <div className="brand-mark" aria-hidden>
              K
            </div>
            <div className="min-w-0">
              <p className="font-heading text-base font-semibold tracking-[0.18em] uppercase">
                kbd.ctrl
              </p>
              <p className="text-xs text-muted-foreground">
                Macropad controls for Codex
              </p>
            </div>
          </div>
          <Badge variant={controlsActive ? "default" : "secondary"}>
            <span
              className="status-dot"
              data-state={controlsActive ? "ready" : "pending"}
              aria-hidden
            />
            {controlsActive ? "Controls active" : "Needs attention"}
          </Badge>
        </header>

        <div className="app-content flex-1">
          {preferences === null ? (
            <div className="flex min-h-[32rem] items-center justify-center">
              <p className="flex items-center gap-2 text-sm text-muted-foreground">
                <IconRefresh className="size-4 animate-spin" aria-hidden />
                Loading kbd.ctrl
              </p>
            </div>
          ) : showOnboarding ? (
            <Onboarding
              key={onboardingRun}
              configurationError={mappingError || backupError}
              configurationPhase={configurationPhase}
              event={latestInputEvent}
              isConfiguring={isBackingUp || isMapping}
              onboarding={preferences.onboarding}
              startAtBeginning={startOnboardingAtBeginning}
              status={status}
              testedControls={testedControls}
              onComplete={completeOnboarding}
              onConfigure={configureForOnboarding}
              onDismiss={dismissOnboarding}
              onRefresh={refreshStatus}
              onRequestControlAccess={requestShortcutControl}
              onRequestInputAccess={requestInputAccess}
              onUpdateState={saveOnboardingState}
            />
          ) : (
            <>
              {!preferences.onboarding.completed && (
                <Alert className="mb-6">
                  <IconShieldCheck />
                  <AlertTitle>Finish setting up your controller</AlertTitle>
                  <AlertDescription>
                    Your progress is saved. Continue when the keypad is
                    available.
                  </AlertDescription>
                  <AlertAction>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => void resumeOnboarding()}
                    >
                      Continue setup
                    </Button>
                  </AlertAction>
                </Alert>
              )}

              <section className="app-hero">
                <div className="max-w-3xl">
                  <p className="eyebrow">kbd.ctrl for Codex</p>
                  <h1 className="mt-3 font-heading text-4xl leading-none font-semibold tracking-tight">
                    Keep your Codex
                    <br />
                    controls close.
                  </h1>
                  <p className="mt-4 max-w-2xl text-sm leading-relaxed text-muted-foreground">
                    Use a USB or Bluetooth macropad to dictate, cancel, confirm,
                    choose a model, and adjust reasoning effort.
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={refreshStatus}
                  disabled={isPending}
                >
                  <IconRefresh data-icon="inline-start" />
                  {isPending ? "Checking" : "Check status"}
                </Button>
              </section>

              {!status.inputMonitoringGranted && (
                <Alert className="mb-6">
                  <IconShieldLock />
                  <AlertTitle>Input Monitoring is required</AlertTitle>
                  <AlertDescription>
                    Allow this signed build to see the controller’s F13 and
                    F16–F20 keys, then relaunch kbd.ctrl.
                  </AlertDescription>
                  <AlertAction>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={requestInputAccess}
                    >
                      Open settings
                    </Button>
                  </AlertAction>
                </Alert>
              )}

              {status.inputMonitoringGranted &&
                !status.controlAccessGranted && (
                  <Alert className="mb-6">
                    <IconShieldLock />
                    <AlertTitle>Accessibility access is required</AlertTitle>
                    <AlertDescription>
                      Allow kbd.ctrl to translate the universal keys into safe,
                      app-specific shortcuts.
                    </AlertDescription>
                    <AlertAction>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={requestShortcutControl}
                      >
                        Open settings
                      </Button>
                    </AlertAction>
                  </Alert>
                )}

              {status.error && (
                <Alert variant="destructive" className="mb-6">
                  <IconActivity />
                  <AlertTitle>USB service inspection failed</AlertTitle>
                  <AlertDescription>{status.error}</AlertDescription>
                </Alert>
              )}

              <Tabs defaultValue="controls">
                <TabsList variant="line" aria-label="kbd.ctrl sections">
                  <TabsTrigger value="controls">Controls</TabsTrigger>
                  <TabsTrigger value="apps">Apps</TabsTrigger>
                  <TabsTrigger value="device">Device setup</TabsTrigger>
                  <TabsTrigger value="settings">Settings</TabsTrigger>
                </TabsList>

                <TabsContent value="controls" className="pt-6">
                  <DevicePad event={latestInputEvent} />
                  <div className="control-status-strip">
                    <div>
                      <p className="eyebrow">Controller</p>
                      <p className="mt-1 text-sm font-medium">
                        {controlsActive ? "Listening" : monitorStatus.message}
                      </p>
                    </div>
                    <div>
                      <p className="eyebrow">Connection</p>
                      <p className="mt-1 flex items-center gap-2 text-sm font-medium">
                        {status.configurationInterfaceVisible ? (
                          <IconUsb aria-hidden />
                        ) : (
                          <IconBluetooth aria-hidden />
                        )}
                        {transportLabel}
                      </p>
                    </div>
                    <div>
                      <p className="eyebrow">Active app</p>
                      <p className="mt-1 text-sm font-medium">
                        {frontmostApplication.name ?? "Desktop"}
                      </p>
                    </div>
                  </div>
                </TabsContent>

                <TabsContent value="apps" className="pt-6">
                  <ProviderSetup />
                </TabsContent>

                <TabsContent value="device" className="pt-6">
                  <div className="grid gap-6 lg:grid-cols-2">
                    <Card>
                      <CardHeader>
                        <CardTitle>Daily connection</CardTitle>
                        <CardDescription>
                          Bluetooth or USB keyboard input
                        </CardDescription>
                        <CardAction>
                          <IconBluetooth aria-hidden />
                        </CardAction>
                      </CardHeader>
                      <CardContent>
                        <StatusRow
                          label="Controls"
                          value={controlsActive ? "Active" : "Needs attention"}
                          state={controlsActive ? "ready" : "pending"}
                        />
                        <Separator />
                        <StatusRow
                          label="Input access"
                          value={
                            status.inputMonitoringGranted
                              ? "Granted"
                              : "Required"
                          }
                          state={
                            status.inputMonitoringGranted ? "ready" : "pending"
                          }
                        />
                        <Separator />
                        <StatusRow
                          label="Control access"
                          value={
                            status.controlAccessGranted ? "Granted" : "Required"
                          }
                          state={
                            status.controlAccessGranted ? "ready" : "pending"
                          }
                        />
                      </CardContent>
                    </Card>

                    <Card>
                      <CardHeader>
                        <CardTitle>USB service connection</CardTitle>
                        <CardDescription>
                          Needed only for mappings, lighting, backup, and
                          restore
                        </CardDescription>
                        <CardAction>
                          <IconUsb aria-hidden />
                        </CardAction>
                      </CardHeader>
                      <CardContent>
                        <StatusRow
                          label="USB"
                          value={
                            status.connected ? "Connected" : "Not connected"
                          }
                          state={status.connected ? "ready" : "pending"}
                        />
                        <Separator />
                        <StatusRow
                          label="Config HID"
                          value={
                            status.configurationInterfaceVisible
                              ? "Ready"
                              : "Connect cable"
                          }
                          state={
                            status.configurationInterfaceVisible
                              ? "ready"
                              : "pending"
                          }
                        />
                      </CardContent>
                      <CardFooter className="flex-col border-t">
                        <Button
                          className="w-full"
                          onClick={createBackup}
                          disabled={
                            !status.configurationInterfaceVisible || isBackingUp
                          }
                        >
                          {isBackingUp ? (
                            <IconRefresh data-icon="inline-start" />
                          ) : (
                            <IconDatabaseExport data-icon="inline-start" />
                          )}
                          {isBackingUp
                            ? "Reading configuration"
                            : "Create backup"}
                        </Button>
                        <Button
                          variant="outline"
                          className="w-full"
                          onClick={applyMapping}
                          disabled={!backup || isMapping}
                        >
                          <IconDeviceFloppy data-icon="inline-start" />
                          {isMapping
                            ? "Writing and verifying"
                            : "Apply universal mapping"}
                        </Button>
                      </CardFooter>
                    </Card>

                    {backup && (
                      <Alert className="lg:col-span-2">
                        <IconShieldCheck />
                        <AlertTitle>Configuration backup verified</AlertTitle>
                        <AlertDescription>
                          Captured {backup.reportCount} reports. Fingerprint{" "}
                          {backup.fingerprintSha256.slice(0, 12)}….
                        </AlertDescription>
                      </Alert>
                    )}
                    {backupError && (
                      <Alert variant="destructive" className="lg:col-span-2">
                        <IconActivity />
                        <AlertTitle>Backup could not be completed</AlertTitle>
                        <AlertDescription>{backupError}</AlertDescription>
                      </Alert>
                    )}
                    {mapping && (
                      <Alert className="lg:col-span-2">
                        <IconShieldCheck />
                        <AlertTitle>Universal mapping verified</AlertTitle>
                        <AlertDescription>
                          Wrote {mapping.writtenReportCount} reports across all
                          three hardware layers.
                        </AlertDescription>
                      </Alert>
                    )}
                    {mappingError && (
                      <Alert variant="destructive" className="lg:col-span-2">
                        <IconActivity />
                        <AlertTitle>Mapping was not applied</AlertTitle>
                        <AlertDescription>{mappingError}</AlertDescription>
                      </Alert>
                    )}
                  </div>
                </TabsContent>

                <TabsContent value="settings" className="pt-6">
                  <ControllerSettings
                    onRunSetup={() => void runOnboarding()}
                    usbServiceAvailable={status.configurationInterfaceVisible}
                  />
                </TabsContent>
              </Tabs>
            </>
          )}
        </div>
        <footer className="app-footer">
          <p>
            Made by{" "}
            <a
              href="https://x.com/jmramox"
              onClick={openExternalLink}
              className="font-medium text-foreground underline-offset-4 hover:underline"
            >
              Juan M. Ramos
            </a>
          </p>
          <nav className="flex items-center gap-1" aria-label="Project links">
            {creatorLinks.map(({ href, label, icon: Icon }) => (
              <a
                key={href}
                href={href}
                onClick={openExternalLink}
                className="app-footer__link"
                aria-label={label}
                title={label}
              >
                <Icon aria-hidden />
              </a>
            ))}
          </nav>
        </footer>
      </main>
    </TooltipProvider>
  )
}

export default App
