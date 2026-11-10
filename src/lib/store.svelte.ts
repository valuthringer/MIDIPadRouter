// Front-end state. The front only edits the config and displays snapshots:
// MIDI traffic never comes through here.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { History } from './history';
import type { Assignment } from './paint';
import type { Config, ConfigError, FullState, PortList, Profile, Snapshot } from './types';

export const app = $state({
  config: null as Config | null,
  ports: { inputs: [], outputs: [] } as PortList,
  snap: null as Snapshot | null,
  platform: '',
  error: null as ConfigError | null,
  warning: null as ConfigError | null,
  shortcutErrors: [] as string[],
  /** Target id used to paint keys; null paints "no application". */
  brush: null as string | null,
  wizard: false,
  settings: false,
});

export const history = new History<Assignment>();

export function profile(): Profile {
  return app.config!.profiles[app.config!.activeProfile];
}

function toError(e: unknown): ConfigError {
  return typeof e === 'object' && e !== null && 'code' in e
    ? (e as ConfigError)
    : { code: 'io', arg: String(e) };
}

function setState(s: FullState) {
  app.config = s.config;
  app.ports = s.ports;
  app.snap = s.snapshot;
  app.platform = s.platform;
  app.shortcutErrors = s.shortcutErrors;
  if (s.warning) app.warning = s.warning;
  if (!profile().targets.some((t) => t.id === app.brush))
    app.brush = profile().targets[0]?.id ?? null;
}

export async function init() {
  const s = await invoke<FullState>('get_state');
  setState(s);
  // The window may have been closed with these on: the panels start closed.
  await Promise.all([setMonitor(false), setPreview(false)]);
  app.wizard = !s.config.settings.setupDone;
  await listen<Snapshot>('snapshot', (e) => (app.snap = e.payload));
  // Profile switched from the tray.
  await listen('config-changed', async () => {
    setState(await invoke<FullState>('get_state'));
    history.clear();
  });
  // Nothing is pushed while hidden: catch up when shown again.
  const refresh = () => !document.hidden && void refreshSnapshot();
  document.addEventListener('visibilitychange', refresh);
  window.addEventListener('focus', refresh);
}

export async function refreshSnapshot() {
  app.snap = await invoke<Snapshot>('get_snapshot');
}

export async function refreshPorts() {
  app.ports = await invoke<PortList>('get_ports');
}

let timer: ReturnType<typeof setTimeout> | undefined;

/** Debounced auto-save. */
export function save(delay = 250) {
  clearTimeout(timer);
  timer = setTimeout(saveNow, delay);
}

export async function saveNow() {
  clearTimeout(timer);
  if (!app.config) return;
  try {
    app.shortcutErrors = await invoke<string[]>('save_config', {
      config: $state.snapshot(app.config),
    });
    app.error = null;
  } catch (e) {
    app.error = toError(e);
  }
}

export const setActive = (index: number) => invoke('set_active', { index });
export const setPreview = (on: boolean) => invoke('set_preview', { on });
export const setMonitor = (on: boolean) => invoke('set_monitor', { on });
export const openHelp = (page: string) => invoke('open_help', { page });

export async function exportProfile(name: string) {
  try {
    await invoke('export_profile', { name });
  } catch (e) {
    app.error = toError(e);
  }
}

export async function importProfile() {
  try {
    const name = await invoke<string | null>('import_profile');
    if (name) setState(await invoke<FullState>('get_state'));
    history.clear();
  } catch (e) {
    app.error = toError(e);
  }
}

/** Replace the assignment of the active profile; `record` pushes an undo step. */
export function editAssignment(next: Assignment, record = true) {
  const p = profile();
  if (record) history.record($state.snapshot(p.assignment));
  p.assignment = next;
  save();
}

export function undo() {
  const p = profile();
  const prev = history.undo($state.snapshot(p.assignment));
  if (prev) {
    p.assignment = prev;
    save();
  }
}

export function redo() {
  const p = profile();
  const next = history.redo($state.snapshot(p.assignment));
  if (next) {
    p.assignment = next;
    save();
  }
}
