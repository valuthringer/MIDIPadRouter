// Key indices mirror devices/launchpad_mini_mk1.rs:
// 0..64 pads (row-major, top-left first), 64..72 scene buttons, 72..80 top buttons.
import type { KeyRef } from './types';

export const KEY_COUNT = 80;
export const ALL_KEYS = Array.from({ length: KEY_COUNT }, (_, i) => i);

export function keyRef(k: number): KeyRef {
  if (k < 64) return { type: 'pad', number: 16 * Math.floor(k / 8) + (k % 8) };
  if (k < 72) return { type: 'pad', number: 16 * (k - 64) + 8 };
  return { type: 'cc', number: 104 + k - 72 };
}

export function keyId(k: number): string {
  const r = keyRef(k);
  return `${r.type}:${r.number}`;
}

export function keyFromRef(r: KeyRef | null): number {
  if (!r) return -1;
  return ALL_KEYS.find((k) => keyId(k) === `${r.type}:${r.number}`) ?? -1;
}

/** Display grid is 9x9: row 0 = top buttons, column 8 = scene buttons, (0, 8) is empty. */
export function cellKey(row: number, col: number): number | null {
  if (row === 0) return col < 8 ? 72 + col : null;
  if (col === 8) return 64 + row - 1;
  return (row - 1) * 8 + col;
}

export function keyCell(k: number): [number, number] {
  if (k < 64) return [Math.floor(k / 8) + 1, k % 8];
  if (k < 72) return [k - 64 + 1, 8];
  return [0, k - 72];
}

const LEVEL = [0, 120, 190, 255];

/** LED velocity (16*green + red + flags) to a CSS colour; null when off. */
export function ledColor(v: number): string | null {
  const red = v & 3;
  const green = (v >> 4) & 3;
  if (!red && !green) return null;
  return `rgb(${LEVEL[red]}, ${LEVEL[green]}, 30)`;
}
