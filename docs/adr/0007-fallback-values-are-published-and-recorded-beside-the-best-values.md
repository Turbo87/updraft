# Fallback values are published and recorded beside the best values

Without an airspeed, Updraft cannot calculate the total-energy Vario or Netto.
It then shows Vertical speed as "Vario (unc.)" and Estimated netto as
"Netto (est.)". The core publishes each method in its own field: `vario` and
`verticalSpeed`, `netto` and `estimatedNetto`, `relativeVario` and
`estimatedRelativeVario`. `vario`, `netto`, and `relativeVario` come only from
a measured or derived airspeed. The frontend chooses the value to show, in the
order fresh best value, fresh fallback, stale best value, then stale fallback.
The infobox and the trail use the same choice.

The flight recording stores both methods in separate columns. A null column
means that its value was unavailable or stale.

## Considered options

- One field with a method flag needs a new protocol concept. The field name
  already identifies the method.
- A stale total-energy `vario` while a current `verticalSpeed` exists lets the
  frontend show a value from before the fallback period. So `vario`, `netto`,
  and `relativeVario` are unavailable, not stale, when no current airspeed
  exists.
- A recording that stores only the shown value without its method was the
  earlier decision. It loses the method, and a later analysis cannot compare
  the two values.

## Consequences

- Each value keeps its own smoothing, so the shown value can jump when the
  method changes. Hysteresis is added only if flight tests show flicker.
- Older recordings have no fallback values.
