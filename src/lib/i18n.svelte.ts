import fr, { type Messages } from './locales/fr';
import en from './locales/en';

export const messages: Record<'fr' | 'en', Messages> = { fr, en };
export type MessageKey = keyof Messages;

export const i18n = $state({ lang: 'fr' as 'fr' | 'en' });

/** Translate `key`, replacing `{name}` placeholders. Unknown keys fall back to French, then the key. */
export function t(key: string, params: Record<string, string | number> = {}): string {
  const text = messages[i18n.lang]?.[key as MessageKey] ?? messages.fr[key as MessageKey] ?? key;
  return text.replace(/\{(\w+)\}/g, (m, p) => (p in params ? String(params[p]) : m));
}
