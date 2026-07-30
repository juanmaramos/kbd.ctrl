export type ControlTarget =
  | "left-key"
  | "middle-key"
  | "right-key"
  | "dial-left"
  | "dial-right"
  | "dial-press"

type ControlMapProps = {
  target: ControlTarget
}

const keyTargets = ["left-key", "middle-key", "right-key"] as const

export function ControlMap({ target }: ControlMapProps) {
  const dialTarget = target.startsWith("dial-")

  return (
    <div className="control-map" data-target={target} aria-hidden="true">
      <div className="control-map__keys">
        {keyTargets.map((keyTarget, index) => (
          <span
            key={keyTarget}
            className="control-map__key"
            data-active={target === keyTarget}
            data-tone={
              index === 0 ? "speak" : index === 1 ? "cancel" : "confirm"
            }
          />
        ))}
      </div>
      <div className="control-map__dial-wrap">
        <span
          className="control-map__gesture control-map__gesture--left"
          data-active={target === "dial-left"}
        >
          ‹
        </span>
        <span
          className="control-map__dial"
          data-active={dialTarget}
          data-pressed={target === "dial-press"}
        />
        <span
          className="control-map__gesture control-map__gesture--right"
          data-active={target === "dial-right"}
        >
          ›
        </span>
      </div>
    </div>
  )
}
