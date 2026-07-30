import { useMemo, useState } from "react"
import {
  IconArrowRight,
  IconBrandOpenai,
  IconCheck,
  IconDeviceFloppy,
  IconExternalLink,
  IconKeyboard,
  IconLockAccess,
  IconRefresh,
  IconUsb,
} from "@tabler/icons-react"

import { ControlMap, type ControlTarget } from "@/components/control-map"
import { DevicePad } from "@/components/device-pad"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Progress, ProgressLabel } from "@/components/ui/progress"
import { Separator } from "@/components/ui/separator"
import type {
  DeviceStatus,
  HardwareInputEvent,
  OnboardingState,
} from "@/lib/device"
import { openCodex } from "@/lib/device"
import { controllerControlCodes } from "@/lib/hardware-setup"

const steps = [
  {
    title: "Connect",
    description: "Find the keypad",
  },
  {
    title: "Permissions",
    description: "Let kbd.ctrl listen and translate",
  },
  {
    title: "Configure",
    description: "Back up and map the controls",
  },
  {
    title: "Codex",
    description: "Assign three shortcuts",
  },
  {
    title: "Test",
    description: "Press every control once",
  },
] as const

type OnboardingProps = {
  configurationError: string | null
  configurationPhase: "backup" | "mapping" | "lighting" | null
  event: HardwareInputEvent | null
  isConfiguring: boolean
  onboarding: OnboardingState
  startAtBeginning: boolean
  status: DeviceStatus
  testedControls: ReadonlySet<string>
  onComplete: () => Promise<void>
  onConfigure: () => Promise<void>
  onDismiss: () => Promise<void>
  onRefresh: () => void
  onRequestControlAccess: () => Promise<void>
  onRequestInputAccess: () => Promise<void>
  onUpdateState: (state: OnboardingState) => Promise<void>
}

function nextIncompleteStep(status: DeviceStatus, onboarding: OnboardingState) {
  const controllerConfigured =
    onboarding.hardwareConfigured && onboarding.lightingConfigured

  if (!status.configurationInterfaceVisible && !onboarding.hardwareConfigured) {
    return 0
  }
  if (!status.inputMonitoringGranted || !status.controlAccessGranted) {
    return 1
  }
  if (!controllerConfigured) {
    return 2
  }
  if (!onboarding.codexConfigured) {
    return 3
  }
  return 4
}

function ShortcutRow({
  control,
  input,
  action,
  target,
}: {
  control: string
  input: string
  action: string
  target: ControlTarget
}) {
  return (
    <div className="flex items-center justify-between gap-6 py-3">
      <div className="flex items-center gap-3">
        <ControlMap target={target} />
        <div>
          <p className="text-sm font-medium">{action}</p>
          <p className="mt-0.5 text-xs text-muted-foreground">{control}</p>
        </div>
      </div>
      <Badge variant="outline" className="font-mono">
        {input}
      </Badge>
    </div>
  )
}

export function Onboarding({
  configurationError,
  configurationPhase,
  event,
  isConfiguring,
  onboarding,
  startAtBeginning,
  status,
  testedControls,
  onComplete,
  onConfigure,
  onDismiss,
  onRefresh,
  onRequestControlAccess,
  onRequestInputAccess,
  onUpdateState,
}: OnboardingProps) {
  const [currentStep, setCurrentStep] = useState(() =>
    startAtBeginning ? 0 : nextIncompleteStep(status, onboarding)
  )
  const [setupError, setSetupError] = useState<string | null>(null)
  const [isConfigureDialogOpen, setIsConfigureDialogOpen] = useState(false)
  const controllerConfigured =
    onboarding.hardwareConfigured && onboarding.lightingConfigured
  const activeStep =
    currentStep === 2 && controllerConfigured && !isConfiguring
      ? 3
      : currentStep

  const stepComplete = useMemo(
    () => [
      status.configurationInterfaceVisible || onboarding.hardwareConfigured,
      status.inputMonitoringGranted && status.controlAccessGranted,
      controllerConfigured,
      onboarding.codexConfigured,
      testedControls.size === controllerControlCodes.length,
    ],
    [
      controllerConfigured,
      onboarding.codexConfigured,
      onboarding.hardwareConfigured,
      status,
      testedControls.size,
    ]
  )
  const completedCount = stepComplete.filter(Boolean).length

  async function configureController() {
    setSetupError(null)
    try {
      await onConfigure()
      await onUpdateState({
        ...onboarding,
        hardwareConfigured: true,
        lightingConfigured: true,
      })
      setCurrentStep(3)
    } catch (error) {
      setSetupError(
        error instanceof Error
          ? error.message
          : "The controller could not be configured."
      )
    }
  }

  async function confirmCodexSetup() {
    await onUpdateState({
      ...onboarding,
      codexConfigured: true,
    })
    setCurrentStep(4)
  }

  return (
    <section className="mx-auto w-full max-w-5xl py-8">
      <div className="mb-8 flex items-start justify-between gap-8">
        <div>
          <p className="eyebrow">First-time setup · about 3 minutes</p>
          <h1 className="mt-3 font-heading text-4xl leading-none font-semibold tracking-tight">
            Set up your controller.
          </h1>
          <p className="mt-4 max-w-2xl text-sm leading-relaxed text-muted-foreground">
            kbd.ctrl will safely map the keypad, connect it to Codex, and check
            every physical control.
          </p>
        </div>
        <Button variant="ghost" size="sm" onClick={() => void onDismiss()}>
          Finish later
        </Button>
      </div>

      <Progress value={(completedCount / steps.length) * 100} className="mb-8">
        <ProgressLabel>Setup progress</ProgressLabel>
        <span className="ml-auto text-sm text-muted-foreground tabular-nums">
          {completedCount} of 5 complete
        </span>
      </Progress>

      <div className="grid gap-6 lg:grid-cols-[15rem_minmax(0,1fr)]">
        <nav aria-label="Setup steps">
          <ol className="border">
            {steps.map((step, index) => (
              <li key={step.title} className="border-b last:border-b-0">
                <button
                  type="button"
                  className="setup-step flex w-full items-start gap-3 px-4 py-4 text-left transition-colors hover:bg-muted/50 disabled:cursor-not-allowed disabled:opacity-45"
                  data-state={
                    stepComplete[index]
                      ? "complete"
                      : activeStep === index
                        ? "current"
                        : "pending"
                  }
                  disabled={index > activeStep && !stepComplete[index]}
                  aria-current={activeStep === index ? "step" : undefined}
                  onClick={() => setCurrentStep(index)}
                >
                  <span
                    className="setup-step__marker mt-0.5 flex size-6 shrink-0 items-center justify-center border text-xs font-semibold"
                    data-state={
                      stepComplete[index]
                        ? "complete"
                        : activeStep === index
                          ? "current"
                          : "pending"
                    }
                  >
                    {stepComplete[index] ? (
                      <IconCheck className="size-4" aria-hidden />
                    ) : (
                      index + 1
                    )}
                  </span>
                  <span>
                    <span className="setup-step__title block text-sm font-medium">
                      {step.title}
                    </span>
                    <span className="mt-0.5 block text-xs leading-relaxed text-muted-foreground">
                      {step.description}
                    </span>
                  </span>
                </button>
              </li>
            ))}
          </ol>
        </nav>

        <div>
          {activeStep === 0 && (
            <Card>
              <CardHeader>
                <CardTitle>Connect by USB for setup</CardTitle>
                <CardDescription>
                  The cable is required once to back up and configure the
                  keypad. Bluetooth works for daily controls afterward.
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="flex items-center justify-between gap-6 border p-5">
                  <div className="flex items-center gap-4">
                    <IconUsb className="size-7" aria-hidden />
                    <div>
                      <p className="text-sm font-medium">MINI-KEYBOARD</p>
                      <p className="mt-1 text-xs text-muted-foreground">
                        USB configuration channel
                      </p>
                    </div>
                  </div>
                  <Badge
                    variant={
                      status.configurationInterfaceVisible
                        ? "default"
                        : "secondary"
                    }
                  >
                    {status.configurationInterfaceVisible
                      ? "Connected"
                      : onboarding.hardwareConfigured
                        ? "Already configured"
                        : "Waiting"}
                  </Badge>
                </div>
                {status.error && (
                  <Alert variant="destructive" className="mt-4">
                    <AlertTitle>
                      Could not inspect the USB connection
                    </AlertTitle>
                    <AlertDescription>{status.error}</AlertDescription>
                  </Alert>
                )}
              </CardContent>
              <CardFooter className="justify-between border-t">
                <Button variant="ghost" onClick={onRefresh}>
                  <IconRefresh data-icon="inline-start" />
                  Check again
                </Button>
                <Button
                  disabled={!stepComplete[0]}
                  onClick={() => setCurrentStep(1)}
                >
                  Continue
                  <IconArrowRight data-icon="inline-end" />
                </Button>
              </CardFooter>
            </Card>
          )}

          {activeStep === 1 && (
            <Card>
              <CardHeader>
                <CardTitle>Allow controller access</CardTitle>
                <CardDescription>
                  macOS requires two approvals. kbd.ctrl only listens for the
                  controller keys and only translates them in supported apps.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex items-center justify-between gap-6 border p-5">
                  <div className="flex items-center gap-4">
                    <IconKeyboard className="size-6" aria-hidden />
                    <div>
                      <p className="text-sm font-medium">Input Monitoring</p>
                      <p className="mt-1 text-xs text-muted-foreground">
                        See F13 and F16–F20 from the keypad
                      </p>
                    </div>
                  </div>
                  {status.inputMonitoringGranted ? (
                    <Badge>
                      <IconCheck data-icon="inline-start" />
                      Granted
                    </Badge>
                  ) : (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => void onRequestInputAccess()}
                    >
                      Open settings
                      <IconExternalLink data-icon="inline-end" />
                    </Button>
                  )}
                </div>
                <div className="flex items-center justify-between gap-6 border p-5">
                  <div className="flex items-center gap-4">
                    <IconLockAccess className="size-6" aria-hidden />
                    <div>
                      <p className="text-sm font-medium">Accessibility</p>
                      <p className="mt-1 text-xs text-muted-foreground">
                        Translate universal keys into Codex shortcuts
                      </p>
                    </div>
                  </div>
                  {status.controlAccessGranted ? (
                    <Badge>
                      <IconCheck data-icon="inline-start" />
                      Granted
                    </Badge>
                  ) : (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => void onRequestControlAccess()}
                    >
                      Open settings
                      <IconExternalLink data-icon="inline-end" />
                    </Button>
                  )}
                </div>
                <p className="text-xs leading-relaxed text-muted-foreground">
                  If macOS asks you to relaunch kbd.ctrl, do so. Setup resumes
                  here automatically.
                </p>
              </CardContent>
              <CardFooter className="justify-between border-t">
                <Button variant="ghost" onClick={onRefresh}>
                  <IconRefresh data-icon="inline-start" />
                  Refresh access
                </Button>
                <Button
                  disabled={!stepComplete[1]}
                  onClick={() => setCurrentStep(controllerConfigured ? 3 : 2)}
                >
                  Continue
                  <IconArrowRight data-icon="inline-end" />
                </Button>
              </CardFooter>
            </Card>
          )}

          {activeStep === 2 && (
            <Card>
              <CardHeader>
                <CardTitle>Configure the keypad</CardTitle>
                <CardDescription>
                  Install the universal controls once. They remain on the keypad
                  for USB and Bluetooth use.
                </CardDescription>
              </CardHeader>
              <CardContent>
                <details className="border bg-muted/35 p-4">
                  <summary className="cursor-pointer text-sm font-medium">
                    What kbd.ctrl changes
                  </summary>
                  <ul className="mt-3 space-y-2 pl-5 text-xs leading-relaxed text-muted-foreground">
                    <li>
                      Creates a local backup of the current configuration.
                    </li>
                    <li>Maps the same six controls to all three layers.</li>
                    <li>Applies the default key colors.</li>
                    <li>Reads everything back before continuing.</li>
                  </ul>
                </details>
                {(setupError || configurationError) && (
                  <Alert variant="destructive" className="mt-4">
                    <AlertTitle>Configuration did not finish</AlertTitle>
                    <AlertDescription>
                      {setupError || configurationError}
                    </AlertDescription>
                  </Alert>
                )}
                {isConfiguring && configurationPhase && (
                  <Alert className="mt-4">
                    <IconRefresh className="animate-spin" />
                    <AlertTitle>
                      {configurationPhase === "backup"
                        ? "Backing up the current keypad"
                        : configurationPhase === "mapping"
                          ? "Writing and verifying the controls"
                          : "Applying and verifying lighting"}
                    </AlertTitle>
                    <AlertDescription>
                      The app remains usable. Keep the keypad connected until
                      this step finishes.
                    </AlertDescription>
                  </Alert>
                )}
              </CardContent>
              <CardFooter className="justify-end border-t">
                {!controllerConfigured && (
                  <AlertDialog
                    open={isConfigureDialogOpen}
                    onOpenChange={setIsConfigureDialogOpen}
                  >
                    <AlertDialogTrigger
                      render={
                        <Button
                          disabled={
                            !status.configurationInterfaceVisible ||
                            isConfiguring
                          }
                        />
                      }
                    >
                      {isConfiguring ? (
                        <IconRefresh data-icon="inline-start" />
                      ) : (
                        <IconDeviceFloppy data-icon="inline-start" />
                      )}
                      {isConfiguring
                        ? "Configuring controller"
                        : "Back up and configure"}
                    </AlertDialogTrigger>
                    <AlertDialogContent>
                      <AlertDialogHeader>
                        <AlertDialogTitle>
                          Configure this keypad?
                        </AlertDialogTitle>
                        <AlertDialogDescription>
                          kbd.ctrl will back up the current configuration before
                          writing the universal controls to all three hardware
                          layers. The write is verified before setup continues.
                        </AlertDialogDescription>
                      </AlertDialogHeader>
                      <AlertDialogFooter>
                        <AlertDialogCancel>Cancel</AlertDialogCancel>
                        <AlertDialogAction
                          onClick={() => {
                            setIsConfigureDialogOpen(false)
                            void configureController()
                          }}
                        >
                          Configure keypad
                        </AlertDialogAction>
                      </AlertDialogFooter>
                    </AlertDialogContent>
                  </AlertDialog>
                )}
              </CardFooter>
            </Card>
          )}

          {activeStep === 3 && (
            <Card>
              <CardHeader>
                <CardTitle>Connect the controls to Codex</CardTitle>
                <CardDescription>
                  Match the left key and both dial directions to three Codex
                  shortcuts. The other controls already work.
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="grid gap-5 sm:grid-cols-2">
                  <div>
                    <div className="flex items-center justify-between pb-2">
                      <p className="text-xs font-semibold tracking-widest uppercase">
                        Set in Codex
                      </p>
                      <Badge variant="secondary">3 shortcuts</Badge>
                    </div>
                    <ShortcutRow
                      control="Press and hold the left key"
                      input="⌃F13"
                      action="Hold to speak"
                      target="left-key"
                    />
                    <Separator />
                    <ShortcutRow
                      control="Turn the dial left"
                      input="F18"
                      action="Decrease reasoning effort"
                      target="dial-left"
                    />
                    <Separator />
                    <ShortcutRow
                      control="Turn the dial right"
                      input="F19"
                      action="Increase reasoning effort"
                      target="dial-right"
                    />
                  </div>
                  <div>
                    <div className="flex items-center justify-between pb-2">
                      <p className="text-xs font-semibold tracking-widest uppercase">
                        Ready automatically
                      </p>
                      <Badge variant="secondary">No setup</Badge>
                    </div>
                    <ShortcutRow
                      control="Press the middle key"
                      input="F16"
                      action="Cancel or reject"
                      target="middle-key"
                    />
                    <Separator />
                    <ShortcutRow
                      control="Press the right key"
                      input="F17"
                      action="Confirm or send"
                      target="right-key"
                    />
                    <Separator />
                    <ShortcutRow
                      control="Press the dial"
                      input="F20"
                      action="Open the model picker"
                      target="dial-press"
                    />
                  </div>
                </div>
                <Alert className="mt-5">
                  <IconBrandOpenai />
                  <AlertTitle>In Codex, open Settings with ⌘,</AlertTitle>
                  <AlertDescription>
                    Choose Keyboard shortcuts and assign the three controls on
                    the left. You can also press ⌘/ and choose Keyboard
                    shortcuts.
                  </AlertDescription>
                </Alert>
              </CardContent>
              <CardFooter className="justify-between border-t">
                <Button variant="outline" onClick={() => void openCodex()}>
                  <IconBrandOpenai data-icon="inline-start" />
                  Open Codex
                  <IconExternalLink data-icon="inline-end" />
                </Button>
                <Button onClick={() => void confirmCodexSetup()}>
                  I’ve assigned them
                  <IconArrowRight data-icon="inline-end" />
                </Button>
              </CardFooter>
            </Card>
          )}

          {activeStep === 4 && (
            <Card>
              <CardHeader>
                <CardTitle>Test every control</CardTitle>
                <CardDescription>
                  Keep this window active and press each key, turn the dial both
                  ways, then click it. The matching control will animate.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-5">
                <DevicePad event={event} />
                <div className="grid grid-cols-3 gap-2 sm:grid-cols-6">
                  {controllerControlCodes.map((control) => (
                    <div
                      key={control}
                      className="flex items-center justify-center gap-2 border px-2 py-3 text-xs font-medium"
                    >
                      {testedControls.has(control) ? (
                        <IconCheck
                          className="size-4 text-primary"
                          aria-hidden
                        />
                      ) : (
                        <span className="size-2 bg-muted-foreground/35" />
                      )}
                      {control}
                    </div>
                  ))}
                </div>
                {testedControls.size < controllerControlCodes.length && (
                  <p className="text-center text-xs text-muted-foreground">
                    {controllerControlCodes.length - testedControls.size}{" "}
                    controls left
                  </p>
                )}
              </CardContent>
              <CardFooter className="justify-end border-t">
                <Button
                  disabled={
                    testedControls.size !== controllerControlCodes.length
                  }
                  onClick={() => void onComplete()}
                >
                  Finish setup
                  <IconCheck data-icon="inline-end" />
                </Button>
              </CardFooter>
            </Card>
          )}
        </div>
      </div>
    </section>
  )
}
