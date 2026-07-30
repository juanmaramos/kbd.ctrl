import { memo } from "react"
import {
  IconBrandOpenai,
  IconCheck,
  IconMessageCircle,
} from "@tabler/icons-react"

import { ControlMap, type ControlTarget } from "@/components/control-map"
import { Badge } from "@/components/ui/badge"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Separator } from "@/components/ui/separator"

type BindingRowProps = {
  control: string
  action: string
  shortcut: string
  setup: string
  target: ControlTarget
}

function BindingRow({
  control,
  action,
  shortcut,
  setup,
  target,
}: BindingRowProps) {
  return (
    <div className="grid grid-cols-[8rem_minmax(0,1fr)] gap-4 py-3">
      <div>
        <div className="mb-2">
          <ControlMap target={target} />
        </div>
        <p className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {control}
        </p>
        <Badge variant="outline" className="mt-1.5 font-mono">
          {shortcut}
        </Badge>
      </div>
      <div className="min-w-0">
        <p className="text-sm font-medium">{action}</p>
        <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
          {setup}
        </p>
      </div>
    </div>
  )
}

export const ProviderSetup = memo(function ProviderSetup() {
  return (
    <div className="grid gap-6 lg:grid-cols-[1.15fr_0.85fr]">
      <Card>
        <CardHeader>
          <CardTitle>Codex</CardTitle>
          <CardDescription>
            Set three shortcuts once; the other controls work automatically
          </CardDescription>
          <CardAction className="flex items-center gap-3">
            <Badge>
              <IconCheck data-icon="inline-start" />
              Verified
            </Badge>
            <IconBrandOpenai aria-hidden />
          </CardAction>
        </CardHeader>
        <CardContent>
          <div className="mb-3 border-y py-4">
            <p className="text-xs font-semibold tracking-widest uppercase">
              Set up three shortcuts
            </p>
            <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
              Press <span className="font-medium text-foreground">⌘,</span> to
              open Codex Settings, then choose Keyboard shortcuts. You can also
              press <span className="font-medium text-foreground">⌘/</span> and
              choose Keyboard shortcuts. Match the physical controls below to
              the three shortcut codes shown.
            </p>
          </div>

          <div className="flex items-center justify-between py-2">
            <p className="text-xs font-semibold tracking-widest uppercase">
              Set in Codex
            </p>
            <Badge variant="secondary">3 shortcuts</Badge>
          </div>
          <BindingRow
            control="Left key"
            action="Speak"
            shortcut="⌃F13"
            setup="Assign Hold-to-dictate hotkey to ⌃F13. Hold the left key while speaking; release inserts a draft without sending it."
            target="left-key"
          />
          <Separator />
          <BindingRow
            control="Turn dial left"
            action="Use less reasoning"
            shortcut="F18"
            setup="Assign Decrease reasoning effort to F18."
            target="dial-left"
          />
          <Separator />
          <BindingRow
            control="Turn dial right"
            action="Use more reasoning"
            shortcut="F19"
            setup="Assign Increase reasoning effort to F19."
            target="dial-right"
          />

          <div className="mt-5 flex items-center justify-between border-t pt-5 pb-2">
            <p className="text-xs font-semibold tracking-widest uppercase">
              Ready automatically
            </p>
            <Badge variant="secondary">No setup</Badge>
          </div>
          <BindingRow
            control="Middle key"
            action="Cancel"
            shortcut="F16"
            setup="Cancels, rejects, or stops the current action while Codex is active."
            target="middle-key"
          />
          <Separator />
          <BindingRow
            control="Right key"
            action="Confirm"
            shortcut="F17"
            setup="Sends a message or approves a request while Codex is active."
            target="right-key"
          />
          <Separator />
          <BindingRow
            control="Press dial"
            action="Choose a model"
            shortcut="F20"
            setup="Opens the Codex model picker."
            target="dial-press"
          />

          <div className="mt-5 border bg-muted/35 p-4">
            <p className="text-sm font-medium">Can I change these controls?</p>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
              The physical layout is fixed in this version. You only need to
              assign Speak and the two dial directions in Codex; kbd.ctrl
              handles the other three controls automatically.
            </p>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Claude Desktop</CardTitle>
          <CardDescription>
            Planned support for Claude Chat and Claude Code
          </CardDescription>
          <CardAction className="flex items-center gap-3">
            <Badge variant="secondary">Coming soon</Badge>
            <IconMessageCircle aria-hidden />
          </CardAction>
        </CardHeader>
        <CardContent>
          <div className="mb-3 border-y py-4">
            <p className="text-xs font-semibold tracking-widest uppercase">
              Not included in this release
            </p>
            <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
              Claude Desktop does not expose the same configurable shortcut
              surface as Codex. The current bridge is experimental and did not
              pass the live hardware test, so this release does not claim
              support yet.
            </p>
          </div>
          <p className="text-sm leading-relaxed text-muted-foreground">
            Codex remains the supported app. Claude will move to Ready only
            after Speak, Cancel, Confirm, model, and effort pass the same
            end-to-end test with both USB and Bluetooth input.
          </p>
        </CardContent>
      </Card>
    </div>
  )
})
