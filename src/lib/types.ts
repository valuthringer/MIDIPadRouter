// Mirrors of the Rust serde models (src-tauri/src/config.rs, midi/router.rs).
export type Mode = 'switch' | 'combined';
export type KeyRef = { type: 'pad' | 'cc'; number: number };

export interface TargetCfg {
  id: string;
  name: string;
  color: string;
  enabled: boolean;
  portToApp: string;
  portFromApp: string;
}

export interface Profile {
  mode: Mode;
  activeTarget: string;
  switchKey: KeyRef | null;
  shortcuts: Record<string, string>;
  targets: TargetCfg[];
  assignment: Record<string, string>;
}

export interface Settings {
  language: 'fr' | 'en';
  autostart: boolean;
  startMinimized: boolean;
  theme: 'auto' | 'light' | 'dark';
  setupDone: boolean;
  virtualPorts: boolean;
}

export interface Config {
  version: number;
  activeProfile: string;
  profiles: Record<string, Profile>;
  settings: Settings;
}

export interface Frame {
  fromLaunchpad: boolean;
  target: number | null;
  bytes: number[];
  len: number;
  routed: boolean;
}

export interface TargetSnap {
  toApp: boolean;
  fromApp: boolean;
  activity: boolean;
  leds: number[];
}

export type LaunchpadStatus = 'ok' | 'missing' | 'busy' | 'unsupported';

export interface Snapshot {
  active: number;
  launchpad: LaunchpadStatus;
  preview: boolean;
  targets: TargetSnap[];
  monitor: Frame[] | null;
}

export interface ConfigError {
  code: string;
  arg: string;
}

export interface PortList {
  inputs: string[];
  outputs: string[];
}

export interface FullState {
  config: Config;
  ports: PortList;
  snapshot: Snapshot;
  platform: string;
  warning: ConfigError | null;
  shortcutErrors: string[];
}
