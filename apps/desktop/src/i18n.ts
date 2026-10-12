// Lintel desktop localization core.
//
// One dependency-light system for the whole App: two checked, complete catalogs
// (zh-CN + en), `{name}` interpolation, locale-aware date/number formatting and
// a finite fallback chain. It is intentionally framework-free so the catalog
// completeness test can import it under `node --experimental-strip-types`.
//
// Resolution model (see resolveLocale):
//   * a manual override (`lintel.language` = 'zh-CN' | 'en') always wins;
//   * otherwise the App-preferred languages returned by the native
//     `app_preferred_languages` command (web/synthetic: `navigator.languages`)
//     are matched in order;
//   * an unknown/empty/invalid preference list falls back to `fallbackLocale`.
// Only the two shipped locales resolve to a catalog; anything else is recorded
// as the raw preference and rendered with the fallback catalog.
export const LOCALES = ['zh-CN', 'en'] as const;
export type Locale = (typeof LOCALES)[number];
export type LanguageSetting = 'system' | Locale;

export const LANGUAGE_STORAGE_KEY = 'lintel.language';
// Fallback for an unsupported/empty/invalid system preference. The native
// Info.plist declares `en` as the development language and ships en/zh-Hans
// localizations, so English is the aligned default when nothing matches.
export const fallbackLocale: Locale = 'en';
export const DEFAULT_LANGUAGE_SETTING: LanguageSetting = 'system';

// The manual-override selector. Option labels are the language endonyms; the
// "system" label is resolved per-locale by the provider (`language.system`).
export const LOCALE_LABELS: Record<Locale, string> = { 'zh-CN': '简体中文', en: 'English' };
export const LANGUAGE_OPTIONS: { value: LanguageSetting; endonym: string }[] = [
  { value: 'system', endonym: '' },
  { value: 'zh-CN', endonym: LOCALE_LABELS['zh-CN'] },
  { value: 'en', endonym: LOCALE_LABELS.en },
];

export type Catalog = { [key: string]: string };

// A loose BCP-47 style tag: 2-8 primary letters + optional subtag(s). The native
// command promises at most 8 identifiers; a defensive cap keeps an over-long
// list from being trusted blindly.
const LANGUAGE_TAG = /^[A-Za-z]{2,8}(-[A-Za-z0-9]{1,8})*$/;
export const MAX_PREFERRED_LANGUAGES = 8;

/** Normalize one identifier; return null when it is not a plausible tag. */
export function normalizeTag(value: unknown): string | null {
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  if (!trimmed || trimmed.length > 64) return null;
  return LANGUAGE_TAG.test(trimmed) ? trimmed : null;
}

/** Keep only plausible identifiers, in order, capped at the finite maximum. */
export function sanitizePreferredLanguages(values: unknown): string[] {
  if (!Array.isArray(values)) return [];
  const out: string[] = [];
  for (const value of values) {
    const tag = normalizeTag(value);
    if (tag && !out.includes(tag)) out.push(tag);
    if (out.length >= MAX_PREFERRED_LANGUAGES) break;
  }
  return out;
}

/** Map a BCP-47-style identifier to a shipped catalog, or null when unsupported. */
export function catalogForTag(tag: string): Locale | null {
  const lower = tag.trim().toLowerCase();
  if (!lower) return null;
  // Chinese (any region/script variant: zh, zh-CN, zh-Hans, zh-TW, zh-Hant…).
  if (lower === 'zh' || lower.startsWith('zh-')) return 'zh-CN';
  // English (any region: en, en-US, en-GB…).
  if (lower === 'en' || lower.startsWith('en-')) return 'en';
  return null;
}

export type Resolution = {
  /** The language preference that produced this render. */
  setting: LanguageSetting;
  /** The catalog in use (always a shipped locale). */
  locale: Locale;
  /** The identifier that matched a catalog, or null when the fallback applied. */
  matched: string | null;
  /** The raw, sanitized preference list consulted in system mode. */
  preferred: string[];
};

/**
 * Resolve the locale for one render. A manual override short-circuits the
 * preference list entirely (so a `languagechange` never overrides it).
 */
export function resolveLocale(
  setting: LanguageSetting | string | null | undefined,
  preferredLanguages: unknown,
  fallback: Locale = fallbackLocale,
): Resolution {
  const pref = sanitizePreferredLanguages(preferredLanguages);
  if (setting === 'zh-CN' || setting === 'en') {
    return { setting, locale: setting, matched: setting, preferred: pref };
  }
  for (const tag of pref) {
    const locale = catalogForTag(tag);
    if (locale) return { setting: 'system', locale, matched: tag, preferred: pref };
  }
  return { setting: 'system', locale: fallback, matched: null, preferred: pref };
}

/** True when the persisted value is a finite, known setting; otherwise `system`. */
export function normalizeLanguageSetting(value: unknown): LanguageSetting {
  return value === 'zh-CN' || value === 'en' || value === 'system' ? value : DEFAULT_LANGUAGE_SETTING;
}

// ---------------------------------------------------------------------------
// Catalog lookup + interpolation
// ---------------------------------------------------------------------------

export type TranslateArgs = Record<string, string | number> | undefined;

/** UI-owned failure identity; resolve wording at render, preserving its args. */
export class LocalizedError extends Error {
  readonly key: string;
  readonly args?: TranslateArgs;
  constructor(key: string, args?: TranslateArgs) {
    super(key);
    this.name = 'LocalizedError';
    this.key = key;
    this.args = args;
  }
}


/**
 * Translate `key` for `locale`, interpolating `{name}` placeholders.
 *
 * The catalog is a *flat* map of dotted string keys (`"nav.work": "…"`), so the
 * lookup is a direct property read — never a traversal of a nested object.
 *
 * Fallback chain (finite): requested locale -> fallback locale -> the key
 * itself. A missing placeholder value is left as the literal `{name}` so an
 * obvious gap is visible rather than silently blank.
 */
export function translate(
  catalogs: Record<Locale, Catalog>,
  locale: Locale,
  key: string,
  args?: TranslateArgs,
): string {
  const requested = catalogs[locale];
  const fallback = catalogs[fallbackLocale];
  const raw = requested && Object.hasOwn(requested, key) ? requested[key]
    : fallback && Object.hasOwn(fallback, key) ? fallback[key] : undefined;
  if (typeof raw !== 'string') return key;
  if (!args) return raw;
  return raw.replace(/\{(\w+)\}/g, (match, name: string) =>
    Object.prototype.hasOwnProperty.call(args, name) ? String(args[name]) : match,
  );
}

// ---------------------------------------------------------------------------
// Locale-aware formatting
// ---------------------------------------------------------------------------

function localeTag(locale: Locale): string {
  // `en` is rendered as en-US so dates are stable regardless of the host's
  // resolved default region; zh-CN is already fully specified.
  return locale === 'en' ? 'en-US' : 'zh-CN';
}

/** Short human date for a receipt/record timestamp (epoch seconds/ms or ISO). */
export function formatDateTime(value: string | number, locale: Locale): string {
  const numeric = Number(value);
  const date = new Date(Number.isFinite(numeric) ? numeric * (numeric < 1e12 ? 1000 : 1) : value);
  if (Number.isNaN(date.getTime())) return typeof value === 'string' ? value : String(value);
  return new Intl.DateTimeFormat(localeTag(locale), { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }).format(date);
}

/** Wall-clock time-of-day, e.g. the inspector's "checked at" label. */
export function formatTime(date: Date, locale: Locale): string {
  return new Intl.DateTimeFormat(localeTag(locale), { hour: '2-digit', minute: '2-digit' }).format(date);
}

/** Group-separated integer. */
export function formatNumber(value: number, locale: Locale): string {
  return new Intl.NumberFormat(localeTag(locale)).format(value);
}
