import { describe, expect, it } from 'vitest';
import { ALL_KEYS, KEY_COUNT, cellKey, keyCell, keyFromRef, keyId, keyRef, ledColor } from './keys';
import { halfKeys, paint, shapeKeys } from './paint';

describe('keys', () => {
  it('matches the Rust key table', () => {
    expect(keyId(0)).toBe('pad:0');
    expect(keyId(9)).toBe('pad:17');
    expect(keyId(64)).toBe('pad:8');
    expect(keyId(71)).toBe('pad:120');
    expect(keyId(72)).toBe('cc:104');
    expect(keyId(79)).toBe('cc:111');
    expect(new Set(ALL_KEYS.map(keyId)).size).toBe(KEY_COUNT);
  });

  it('round-trips refs and grid cells', () => {
    for (const k of ALL_KEYS) {
      expect(keyFromRef(keyRef(k))).toBe(k);
      const [r, c] = keyCell(k);
      expect(cellKey(r, c)).toBe(k);
    }
    expect(cellKey(0, 8)).toBeNull();
    expect(keyFromRef(null)).toBe(-1);
  });

  it('decodes LED velocities', () => {
    expect(ledColor(0x0c)).toBeNull();
    expect(ledColor(0)).toBeNull();
    expect(ledColor(0x0f)).toBe('rgb(255, 0, 30)');
    expect(ledColor(0x3c)).toBe('rgb(0, 255, 30)');
  });
});

describe('paint', () => {
  it('paints, erases and skips the reserved key', () => {
    let a = paint({}, [0, 1, 79], 'a', 79);
    expect(a).toEqual({ 'pad:0': 'a', 'pad:1': 'a' });
    a = paint(a, [1], null, 79);
    expect(a).toEqual({ 'pad:0': 'a' });
  });

  it('does not mutate its input', () => {
    const a = { 'pad:0': 'a' };
    paint(a, [0], 'b', -1);
    expect(a).toEqual({ 'pad:0': 'a' });
  });

  it('brush shapes cover display rows and columns', () => {
    expect(shapeKeys(0, 'key')).toEqual([0]);
    expect(shapeKeys(0, 'row')).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 64]);
    expect(shapeKeys(0, 'column')).toEqual([72, 0, 8, 16, 24, 32, 40, 48, 56]);
    expect(shapeKeys(72, 'row')).toHaveLength(8);
    expect(shapeKeys(64, 'column')).toHaveLength(8);
  });

  it('halves split all 80 keys', () => {
    const l = halfKeys('left');
    const r = halfKeys('right');
    expect(l.length + r.length).toBe(KEY_COUNT);
    expect(l).toContain(72);
    expect(r).toContain(64);
  });
});
