import { describe, expect, it } from 'vitest';
import { i18n, messages, t } from './i18n.svelte';

describe('i18n', () => {
  it('French and English have the same keys, none empty', () => {
    expect(Object.keys(messages.en).sort()).toEqual(Object.keys(messages.fr).sort());
    for (const lang of ['fr', 'en'] as const)
      for (const v of Object.values(messages[lang])) expect(v.trim()).not.toBe('');
  });

  it('defaults to French, switches, interpolates', () => {
    expect(t('mode.switch')).toBe('Bascule');
    i18n.lang = 'en';
    expect(t('mode.switch')).toBe('Switch');
    expect(t('grid.pad', { row: 2, col: 3 })).toBe('Pad row 2, column 3');
    expect(t('nope.missing')).toBe('nope.missing');
    i18n.lang = 'fr';
  });
});
