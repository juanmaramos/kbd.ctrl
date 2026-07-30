import { useCallback, useEffect, useState } from "react"
import {
  IconBolt,
  IconBulb,
  IconDeviceDesktop,
  IconRefresh,
  IconRoute,
} from "@tabler/icons-react"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Separator } from "@/components/ui/separator"
import { Switch } from "@/components/ui/switch"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import {
  applyRgbProfile,
  defaultRoleColors,
  readAppPreferences,
  readRgbStatus,
  updateDockVisibility,
  updateLaunchAtLogin,
  type AppPreferences,
  type RgbStatus,
} from "@/lib/device"

const roleLabels = ["Speak", "Cancel", "Confirm"]

type ControllerSettingsProps = {
  lightingConfigured: boolean
  onRunSetup: () => void
  usbServiceAvailable: boolean
}

export function ControllerSettings({
  lightingConfigured,
  onRunSetup,
  usbServiceAvailable,
}: ControllerSettingsProps) {
  const [preferences, setPreferences] = useState<AppPreferences>({
    showDockIcon: false,
    launchAtLogin: false,
    onboarding: {
      completed: false,
      dismissed: false,
      hardwareConfigured: false,
      lightingConfigured: false,
      codexConfigured: false,
    },
  })
  const [rgbStatus, setRgbStatus] = useState<RgbStatus>({
    available: false,
    profile: null,
    modes: [],
    roleColors: defaultRoleColors,
    message: "Connect by USB to inspect lighting.",
  })
  const [draftProfile, setDraftProfile] = useState<"reactive" | "static">(
    "reactive"
  )
  const [draftColors, setDraftColors] = useState<string[]>(defaultRoleColors)
  const [isLoading, setIsLoading] = useState(true)
  const [isSavingPreference, setIsSavingPreference] = useState(false)
  const [isSavingRgb, setIsSavingRgb] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [rgbBackupPath, setRgbBackupPath] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      const [nextPreferences, nextRgbStatus] = await Promise.all([
        readAppPreferences(),
        readRgbStatus(),
      ])
      setPreferences(nextPreferences)
      setRgbStatus(nextRgbStatus)
      setDraftColors(nextRgbStatus.roleColors)
      if (
        nextRgbStatus.profile === "reactive" ||
        nextRgbStatus.profile === "static"
      ) {
        setDraftProfile(nextRgbStatus.profile)
      }
    } catch (refreshError) {
      setError(
        refreshError instanceof Error
          ? refreshError.message
          : "Settings could not be loaded."
      )
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    let cancelled = false

    void Promise.all([readAppPreferences(), readRgbStatus()])
      .then(([nextPreferences, nextRgbStatus]) => {
        if (!cancelled) {
          setPreferences(nextPreferences)
          setRgbStatus(nextRgbStatus)
          setDraftColors(nextRgbStatus.roleColors)
          if (
            nextRgbStatus.profile === "reactive" ||
            nextRgbStatus.profile === "static"
          ) {
            setDraftProfile(nextRgbStatus.profile)
          }
        }
      })
      .catch((refreshError: unknown) => {
        if (!cancelled) {
          setError(
            refreshError instanceof Error
              ? refreshError.message
              : "Settings could not be loaded."
          )
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [lightingConfigured, usbServiceAvailable])

  const setDockIcon = useCallback(async (visible: boolean) => {
    setIsSavingPreference(true)
    setError(null)
    try {
      setPreferences(await updateDockVisibility(visible))
    } catch (saveError) {
      setError(
        saveError instanceof Error
          ? saveError.message
          : "Dock visibility could not be changed."
      )
    } finally {
      setIsSavingPreference(false)
    }
  }, [])

  const setLaunchAtLogin = useCallback(async (enabled: boolean) => {
    setIsSavingPreference(true)
    setError(null)
    try {
      setPreferences(await updateLaunchAtLogin(enabled))
    } catch (saveError) {
      setError(
        saveError instanceof Error
          ? saveError.message
          : "Launch at login could not be changed."
      )
    } finally {
      setIsSavingPreference(false)
    }
  }, [])

  const saveLightingProfile = useCallback(async () => {
    setIsSavingRgb(true)
    setError(null)
    try {
      const summary = await applyRgbProfile(draftProfile, draftColors)
      setRgbBackupPath(summary.backupPath)
      const nextStatus = await readRgbStatus()
      setRgbStatus(nextStatus)
      setDraftColors(nextStatus.roleColors)
    } catch (saveError) {
      setError(
        saveError instanceof Error
          ? saveError.message
          : "The lighting profile could not be applied."
      )
    } finally {
      setIsSavingRgb(false)
    }
  }, [draftColors, draftProfile])

  const updateDraftColor = useCallback((index: number, color: string) => {
    setDraftColors((current) =>
      current.map((currentColor, currentIndex) =>
        currentIndex === index ? color.toUpperCase() : currentColor
      )
    )
  }, [])

  return (
    <div className="grid gap-6 lg:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>Background agent</CardTitle>
          <CardDescription>
            Keep app-aware translations available without keeping a window open
          </CardDescription>
          <CardAction>
            <IconDeviceDesktop aria-hidden />
          </CardAction>
        </CardHeader>
        <CardContent>
          <FieldGroup className="gap-6">
            <Field orientation="horizontal">
              <FieldContent>
                <FieldLabel htmlFor="launch-at-login">
                  Launch at login
                </FieldLabel>
                <FieldDescription>
                  Recommended. Starts quietly in the menu bar so the controller
                  is ready before your AI apps.
                </FieldDescription>
              </FieldContent>
              <Switch
                id="launch-at-login"
                checked={preferences.launchAtLogin}
                disabled={isLoading || isSavingPreference}
                onCheckedChange={setLaunchAtLogin}
                aria-label="Launch kbd.ctrl at login"
              />
            </Field>
            <Separator />
            <Field orientation="horizontal">
              <FieldContent>
                <FieldLabel htmlFor="show-dock-icon">Show in Dock</FieldLabel>
                <FieldDescription>
                  Off keeps kbd.ctrl in the menu bar only. Closing the window
                  does not stop the agent. Menu-bar organizers may put its dice
                  icon in Hidden the first time.
                </FieldDescription>
              </FieldContent>
              <Switch
                id="show-dock-icon"
                checked={preferences.showDockIcon}
                disabled={isLoading || isSavingPreference}
                onCheckedChange={setDockIcon}
                aria-label="Show kbd.ctrl in the Dock"
              />
            </Field>
          </FieldGroup>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Key lighting</CardTitle>
          <CardDescription>
            Available while the keypad is connected by USB
          </CardDescription>
          <CardAction className="flex items-center gap-3">
            <Badge variant={rgbStatus.available ? "default" : "secondary"}>
              {rgbStatus.available ? "USB ready" : "USB required"}
            </Badge>
            <IconBulb aria-hidden />
          </CardAction>
        </CardHeader>
        <CardContent className="flex flex-col gap-5">
          <div className="rgb-role-preview" aria-label="Role colors">
            {roleLabels.map((label, index) => (
              <div key={label}>
                <span
                  className="rgb-role-preview__swatch"
                  style={{
                    backgroundColor:
                      draftColors[index] ?? defaultRoleColors[index],
                  }}
                  aria-hidden
                />
                <span>{label}</span>
              </div>
            ))}
          </div>

          <FieldGroup className="gap-6">
            <FieldSet>
              <FieldLegend>Lighting behavior</FieldLegend>
              <ToggleGroup
                variant="outline"
                spacing={0}
                value={[draftProfile]}
                disabled={!rgbStatus.available || isSavingRgb}
                onValueChange={(values) => {
                  const value = values[0]
                  if (value === "reactive" || value === "static") {
                    setDraftProfile(value)
                  }
                }}
                aria-label="Lighting behavior"
              >
                <ToggleGroupItem value="reactive">
                  <IconBolt data-icon="inline-start" />
                  Pulse on press
                </ToggleGroupItem>
                <ToggleGroupItem value="static">
                  <IconBulb data-icon="inline-start" />
                  Always on
                </ToggleGroupItem>
              </ToggleGroup>
              <FieldDescription>
                Pulse on press briefly lights the control when you use it.
                Always on keeps each assigned color visible while USB is
                connected.
              </FieldDescription>
            </FieldSet>

            <FieldSet>
              <FieldLegend>Control colors</FieldLegend>
              <FieldDescription>
                This keypad has RGB lighting under its three keys. The dial and
                layer indicators are not color-addressable.
              </FieldDescription>
              <FieldGroup className="gap-4">
                {roleLabels.map((label, index) => (
                  <Field key={label} orientation="horizontal">
                    <FieldContent>
                      <FieldLabel htmlFor={`rgb-${label.toLowerCase()}`}>
                        {label}
                      </FieldLabel>
                      <FieldDescription>
                        {draftColors[index] ?? defaultRoleColors[index]}
                      </FieldDescription>
                    </FieldContent>
                    <Input
                      id={`rgb-${label.toLowerCase()}`}
                      type="color"
                      className="w-14"
                      value={draftColors[index] ?? defaultRoleColors[index]}
                      disabled={!rgbStatus.available || isSavingRgb}
                      onChange={(event) =>
                        updateDraftColor(index, event.currentTarget.value)
                      }
                      aria-label={`${label} color`}
                    />
                  </Field>
                ))}
              </FieldGroup>
            </FieldSet>
          </FieldGroup>

          <Button
            onClick={() => void saveLightingProfile()}
            disabled={!rgbStatus.available || isSavingRgb}
          >
            {isSavingRgb ? (
              <IconRefresh data-icon="inline-start" />
            ) : (
              <IconBulb data-icon="inline-start" />
            )}
            {isSavingRgb ? "Applying and verifying" : "Apply lighting"}
          </Button>

          {!rgbStatus.available && (
            <p className="text-sm leading-relaxed text-muted-foreground">
              {rgbStatus.message}
            </p>
          )}

          <p className="text-xs leading-relaxed text-muted-foreground">
            The firmware controls pulse timing; it cannot stay lit for an entire
            dictation. Codex also does not publish a reliable pending-approval
            signal, so lighting represents physical control roles rather than
            live app state.
          </p>

          {rgbBackupPath && (
            <p className="text-xs leading-relaxed text-muted-foreground">
              Original lighting backed up before the verified write.
            </p>
          )}
        </CardContent>
      </Card>

      {error && (
        <Alert variant="destructive" className="lg:col-span-2">
          <IconBulb />
          <AlertTitle>Setting not changed</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <div className="flex items-center justify-between gap-4 lg:col-span-2">
        <Button variant="outline" size="sm" onClick={onRunSetup}>
          <IconRoute data-icon="inline-start" />
          Run setup again
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={refresh}
          disabled={isLoading}
        >
          <IconRefresh data-icon="inline-start" />
          {isLoading ? "Checking" : "Refresh settings"}
        </Button>
      </div>
    </div>
  )
}
