import { beforeEach, describe, expect, it, vi } from 'vitest';

const calls: { cmd: string; args: unknown }[] = [];
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string, args: unknown) => {
    calls.push({ cmd, args });
    return [];
  }),
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

const { app, editAssignment, history, profile, redo, saveNow, undo } =
  await import('./store.svelte');

beforeEach(() => {
  calls.length = 0;
  history.clear();
  app.config = {
    version: 1,
    activeProfile: 'p',
    profiles: {
      p: {
        mode: 'combined',
        activeTarget: 'a',
        switchKey: null,
        shortcuts: {},
        targets: [],
        assignment: {},
      },
    },
    settings: {
      language: 'fr',
      autostart: false,
      startMinimized: false,
      theme: 'auto',
      setupDone: true,
      virtualPorts: false,
    },
  };
});

describe('store', () => {
  it('assignment edits are undoable and saved', async () => {
    editAssignment({ 'pad:0': 'a' });
    editAssignment({ 'pad:0': 'a', 'pad:1': 'a' }, false); // same stroke
    editAssignment({});
    undo();
    expect(profile().assignment).toEqual({ 'pad:0': 'a', 'pad:1': 'a' });
    undo();
    expect(profile().assignment).toEqual({});
    redo();
    expect(profile().assignment).toEqual({ 'pad:0': 'a', 'pad:1': 'a' });
    await saveNow();
    const last = calls.at(-1)!;
    expect(last.cmd).toBe('save_config');
    expect(
      (last.args as { config: { profiles: { p: { assignment: object } } } }).config.profiles.p
        .assignment,
    ).toEqual({ 'pad:0': 'a', 'pad:1': 'a' });
  });

  it('a rejected save shows a clear error', async () => {
    const core = await import('@tauri-apps/api/core');
    vi.mocked(core.invoke).mockRejectedValueOnce({ code: 'duplicatePort', arg: 'X' });
    await saveNow();
    expect(app.error).toEqual({ code: 'duplicatePort', arg: 'X' });
  });
});
