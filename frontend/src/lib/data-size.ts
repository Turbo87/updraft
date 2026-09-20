import { getLocale } from './paraglide/runtime.js';

export function formatDataSize(bytes: number) {
  return new Intl.NumberFormat(getLocale(), {
    style: 'unit',
    unit: 'megabyte',
    maximumFractionDigits: 1,
  })
    .format(bytes / 1_000_000)
    .replaceAll(' ', '\u00a0');
}
