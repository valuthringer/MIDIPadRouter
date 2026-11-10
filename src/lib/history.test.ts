import { describe, expect, it } from 'vitest';
import { History } from './history';

describe('History', () => {
  it('undoes and redoes', () => {
    const h = new History<number>();
    h.record(1); // state before change to 2
    h.record(2); // state before change to 3
    expect(h.undo(3)).toBe(2);
    expect(h.undo(2)).toBe(1);
    expect(h.undo(1)).toBeUndefined();
    expect(h.redo(1)).toBe(2);
    expect(h.redo(2)).toBe(3);
    expect(h.redo(3)).toBeUndefined();
  });

  it('a new change clears redo, and the stack is bounded', () => {
    const h = new History<number>(2);
    h.record(1);
    h.undo(2);
    h.record(5);
    expect(h.redo(6)).toBeUndefined();
    h.record(6);
    h.record(7);
    expect(h.undo(8)).toBe(7);
    expect(h.undo(7)).toBe(6);
    expect(h.undo(6)).toBeUndefined();
  });
});
