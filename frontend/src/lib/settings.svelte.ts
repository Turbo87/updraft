// Settings state lives in a rune-backed module, not in the dialog: each screen
// is its own route/component that mounts and unmounts as you navigate, so values
// that must survive navigation live here. These are placeholders until the core
// owns settings state (optimistic UI is fine for harmless toggles).

export const settings = $state({
  units: 'metric' as 'metric' | 'imperial',
  orientation: 'north-up' as 'north-up' | 'track-up',
  airspaceLabels: true,
});
