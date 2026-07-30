import { memo, type ComponentType } from "react"
import {
  IconCheck,
  IconChevronLeft,
  IconChevronRight,
  IconMicrophone,
  IconPlayerStop,
  IconRotateClockwise,
} from "@tabler/icons-react"

import type { HardwareInputEvent } from "@/lib/device"

type ControlProps = {
  icon: ComponentType<{ "aria-hidden"?: boolean }>
  input: string
  inputState: HardwareInputEvent["state"] | "idle"
  label: string
  note: string
  tone: "voice" | "confirm" | "cancel"
}

function PadControl({
  icon: Icon,
  input,
  inputState,
  label,
  note,
  tone,
}: ControlProps) {
  return (
    <button
      type="button"
      className="device-key"
      data-tone={tone}
      data-input-state={inputState}
      aria-pressed={inputState === "down"}
      aria-label={`${label}: ${note}`}
    >
      <span className="device-key__input">{input}</span>
      <span className="device-key__icon">
        <Icon aria-hidden />
      </span>
      <span className="device-key__label">{label}</span>
      <span className="device-key__note">{note}</span>
    </button>
  )
}

type DevicePadProps = {
  event: HardwareInputEvent | null
}

function inputState(
  event: HardwareInputEvent | null,
  input: string
): HardwareInputEvent["state"] | "idle" {
  return event?.control === input ? event.state : "idle"
}

export const DevicePad = memo(function DevicePad({ event }: DevicePadProps) {
  const dialInput =
    event?.control === "F18"
      ? "less"
      : event?.control === "F19"
        ? "more"
        : event?.control === "F20"
          ? "press"
          : "idle"

  return (
    <div className="device-stage" aria-label="Universal keypad controls">
      <span className="sr-only" aria-live="polite">
        {event ? `${event.control} ${event.state}` : ""}
      </span>
      <div className="device-stage__meta">
        <span>Universal controls</span>
        <span>Same on every layer</span>
      </div>

      <div className="device-shell">
        <div className="device-shell__rail" aria-hidden>
          <span />
          <span />
          <span />
        </div>

        <div className="device-shell__controls">
          <PadControl
            icon={IconMicrophone}
            input="F13 → ⌃F13"
            inputState={inputState(event, "F13")}
            label="Speak"
            note="Hold · release to finish"
            tone="voice"
          />
          <PadControl
            icon={IconPlayerStop}
            input="F16"
            inputState={inputState(event, "F16")}
            label="Cancel"
            note="Reject · stop"
            tone="cancel"
          />
          <PadControl
            icon={IconCheck}
            input="F17"
            inputState={inputState(event, "F17")}
            label="Confirm"
            note="Send · approve once"
            tone="confirm"
          />

          <div
            className="device-dial"
            data-input={dialInput}
            data-input-state={
              dialInput === "idle" ? "idle" : (event?.state ?? "idle")
            }
            aria-label="Model and effort dial"
          >
            <div className="device-dial__cap">
              <IconRotateClockwise aria-hidden />
            </div>
            <div className="device-dial__legend">
              <span>
                <IconChevronLeft aria-hidden /> Less
              </span>
              <strong>Effort</strong>
              <span>
                More <IconChevronRight aria-hidden />
              </span>
            </div>
            <span className="device-dial__click">Click · Models</span>
          </div>
        </div>
      </div>
    </div>
  )
})
