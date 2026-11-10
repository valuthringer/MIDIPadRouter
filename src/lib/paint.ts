import { ALL_KEYS, cellKey, keyCell, keyId } from './keys';

export type Assignment = Record<string, string>;
export type Shape = 'key' | 'row' | 'column';

/** Keys hit by one click with the given brush shape (same display row / column). */
export function shapeKeys(k: number, shape: Shape): number[] {
  const [row, col] = keyCell(k);
  if (shape === 'key') return [k];
  const cells =
    shape === 'row'
      ? Array.from({ length: 9 }, (_, c) => cellKey(row, c))
      : Array.from({ length: 9 }, (_, r) => cellKey(r, col));
  return cells.filter((x): x is number => x !== null);
}

/** Left half = columns 0-3 (pads and top buttons); right half = the rest, scene included. */
export function halfKeys(side: 'left' | 'right'): number[] {
  return ALL_KEYS.filter((k) => keyCell(k)[1] < 4 === (side === 'left'));
}

/** Returns a new assignment where `keys` belong to `target` (null = nobody). */
export function paint(
  a: Assignment,
  keys: number[],
  target: string | null,
  reserved: number,
): Assignment {
  const out = { ...a };
  for (const k of keys) {
    if (k === reserved) continue; // the switch key is never assignable
    const id = keyId(k);
    if (target) out[id] = target;
    else delete out[id];
  }
  return out;
}
