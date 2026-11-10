import type { TargetCfg } from './types';

export const MAX_TARGETS = 5;
const IDS = ['a', 'b', 'c', 'd', 'e'];
// Close to the Launchpad's own colours (green, red, amber, yellow, orange).
const COLORS = ['#22c55e', '#ef4444', '#f59e0b', '#eab308', '#f97316'];

/** Default port names, the ones the setup assistant asks to create. */
export function defaultPorts(id: string) {
  const u = id.toUpperCase();
  return { portToApp: `MIDIPadRouter ${u} in`, portFromApp: `MIDIPadRouter ${u} out` };
}

export function newTarget(
  existing: TargetCfg[],
  t: (key: string, params?: Record<string, string>) => string,
): TargetCfg {
  const id = IDS.find((x) => !existing.some((e) => e.id === x)) ?? 'e';
  return {
    id,
    name: t('targets.defaultName', { n: id.toUpperCase() }),
    color: COLORS[IDS.indexOf(id)],
    enabled: true,
    ...defaultPorts(id),
  };
}
