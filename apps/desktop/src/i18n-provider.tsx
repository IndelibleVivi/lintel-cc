import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import {
  DEFAULT_LANGUAGE_SETTING, LANGUAGE_STORAGE_KEY, LOCALE_LABELS, fallbackLocale,
  formatDateTime, formatNumber, formatTime, normalizeLanguageSetting, resolveLocale, sanitizePreferredLanguages,
  translate, type LanguageSetting, type Locale, type Resolution, type TranslateArgs,
} from './i18n';
import { zhCN } from './i18n.catalog.zh-CN';
import { en } from './i18n.catalog.en';

const catalogs = { 'zh-CN': zhCN, en } as const;

export type Translator = (key: string, args?: TranslateArgs) => string;
export type LanguageContextValue = {
  locale: Locale;
  setting: LanguageSetting;
  matched: string | null;
  preferred: string[];
  setLanguage: (setting: LanguageSetting) => void;
  t: Translator;
  formatDateTime: (value: string | number) => string;
  formatTime: (date: Date) => string;
  formatNumber: (value: number) => string;
  localeLabel: (locale: Locale) => string;
};

export const LanguageContext = createContext<LanguageContextValue | null>(null);

function readStoredSetting(): LanguageSetting {
  try {
    return normalizeLanguageSetting(window.localStorage.getItem(LANGUAGE_STORAGE_KEY));
  } catch {
    // Storage may be unavailable (private mode, disabled). Fall back to system
    // and never fail a render over a preference read.
    return DEFAULT_LANGUAGE_SETTING;
  }
}

function persistSetting(setting: LanguageSetting): void {
  try {
    if (setting === 'system') window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
    else window.localStorage.setItem(LANGUAGE_STORAGE_KEY, setting);
  } catch {
    // Persistence is optional; the in-memory setting still drives this session.
  }
}

/**
 * Read the App-preferred language identifiers.
 *
 * Returns `null` when the native command is unavailable or fails — the caller
 * keeps the fallback catalog and leaves native evidence unset rather than
 * inventing one from `navigator.languages`. In web/synthetic mode (no bridge)
 * the finite `navigator.languages` list is the documented fallback.
 */
export async function fetchPreferredLanguages(): Promise<string[] | null> {
  if (isTauri()) {
    try {
      const value = await invoke<'app_preferred_languages'>('app_preferred_languages') as unknown;
      return sanitizePreferredLanguages(value);
    } catch {
      return null;
    }
  }
  return sanitizePreferredLanguages(navigatorLanguageList());
}

function navigatorLanguageList(): string[] {
  const values = [...(navigator.languages ?? []), navigator.language];
  return values.filter((value): value is string => typeof value === 'string' && value.length > 0);
}

/**
 * Resolve the initial language *before* the App mounts so the first paint is in
 * the correct language (no Chinese flash on an English system). `preferred` is
 * `null` when the native command is unavailable, keeping the fallback catalog.
 */
export async function resolveInitialLanguage(): Promise<{ setting: LanguageSetting; preferred: string[] | null }> {
  const setting = readStoredSetting();
  const preferred = setting === 'system' ? await fetchPreferredLanguages() : null;
  return { setting, preferred };
}

export function LanguageProvider({ initialSetting, initialPreferred, children }: {
  initialSetting: LanguageSetting;
  initialPreferred: string[] | null;
  children: ReactNode;
}) {
  const [setting, setSettingState] = useState<LanguageSetting>(() => normalizeLanguageSetting(initialSetting));
  // App-preferred identifiers, or null when unavailable. `null` keeps the
  // fallback locale and never fabricates native preference evidence.
  const [preferred, setPreferred] = useState<string[] | null>(initialPreferred);
  // StrictMode runs setup->cleanup->setup on mount; the setup must set this back
  // to true so the second setup is not treated as unmounted.
  const mounted = useRef(true);
  // A monotonically increasing token: only the latest preference read may apply,
  // so out-of-order `languagechange`/switch reads cannot clobber a newer result.
  // Bumped on every read AND on cleanup, invalidating any in-flight read.
  const preferenceRequest = useRef(0);
  // Menu bookkeeping tracks the *desired* locale and the last *applied* (i.e.
  // successfully invoked) locale separately, so a failed invoke is retried next
  // time and the newest desired locale always wins.
  const desiredMenuLocale = useRef<Locale | null>(null);
  const appliedMenuLocale = useRef<Locale | null>(null);
  const menuQueue = useRef<Promise<void>>(Promise.resolve());

  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; preferenceRequest.current += 1; };
  }, []);

  const resolution: Resolution = useMemo(
    () => resolveLocale(setting, preferred ?? [], fallbackLocale),
    [setting, preferred],
  );

  // Read native preferences and, if this is still the latest request, apply the
  // result. `onApplied` runs only for the winning read (used to apply the
  // coherent System switch).
  const readPreferred = useCallback((onApplied?: (value: string[] | null) => void) => {
    const token = ++preferenceRequest.current;
    return fetchPreferredLanguages().then(value => {
      if (!mounted.current || token !== preferenceRequest.current) return;
      onApplied?.(value);
      return value;
    });
  }, []);

  // Push the menu language, serialized so the newest desired locale wins even if
  // an earlier native invocation resolves later; a failure leaves `applied`
  // unset so a later render/switch retries.
  const pushMenuLanguage = useCallback((locale: Locale) => {
    if (!isTauri()) return;
    desiredMenuLocale.current = locale;
    menuQueue.current = menuQueue.current.then(() => {
      if (!mounted.current || desiredMenuLocale.current !== locale) return;
      if (appliedMenuLocale.current === locale) return;
      return invoke('app_set_language', { language: locale })
        .then(() => { appliedMenuLocale.current = locale; })
        .catch(() => { /* presentation nicety; never blocks the UI or preference */ });
    }).then(() => undefined);
  }, []);

  // On first mount and whenever the resolved locale changes, align the native
  // menu labels with the rendered UI (native mode only).
  useEffect(() => { pushMenuLanguage(resolution.locale); }, [resolution.locale, pushMenuLanguage]);

  // `languagechange` refreshes preferred languages only in System mode; a manual
  // override ignores it so it is never silently replaced.
  useEffect(() => {
    if (setting !== 'system') return;
    const refresh = () => readPreferred(setPreferred);
    window.addEventListener('languagechange', refresh);
    return () => window.removeEventListener('languagechange', refresh);
  }, [setting, readPreferred]);

  const setLanguage = useCallback((next: LanguageSetting) => {
    const normalized = normalizeLanguageSetting(next);
    persistSetting(normalized);
    if (normalized !== 'system') {
      // A manual selection wins immediately and invalidates any pending System
      // read, so a late preference response cannot override it.
      preferenceRequest.current += 1;
      setSettingState(normalized);
      return;
    }
    // Switching back to System: keep the current rendered locale while the
    // fresh native preference read is pending, then apply setting + preferred
    // together so a stale/empty list never causes a fallback flash. The manual
    // setting is committed with the same winning read; a manual selection during
    // the pending read supersedes it.
    const token = ++preferenceRequest.current;
    const commitSystem = () => { if (mounted.current && token === preferenceRequest.current) setSettingState('system'); };
    // In web/synthetic mode (no native bridge) the browser list resolves
    // synchronously into the next microtask; still go through the token guard.
    fetchPreferredLanguages().then(value => {
      if (!mounted.current || token !== preferenceRequest.current) return;
      setPreferred(value);
      commitSystem();
    });
  }, []);

  // The visible document language follows the resolved catalog so assistive
  // technology and font shaping match what is rendered.
  useEffect(() => { document.documentElement.lang = resolution.locale; }, [resolution.locale]);

  const value = useMemo<LanguageContextValue>(() => {
    const { locale } = resolution;
    const t: Translator = (key, args) => translate(catalogs, locale, key, args);
    return {
      locale, setting: resolution.setting, matched: resolution.matched, preferred: resolution.preferred,
      setLanguage, t,
      formatDateTime: value => formatDateTime(value, locale),
      formatTime: date => formatTime(date, locale),
      formatNumber: value => formatNumber(value, locale),
      localeLabel: locale => LOCALE_LABELS[locale],
    };
  }, [resolution, setLanguage]);

  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}

export function useI18n(): LanguageContextValue {
  const value = useContext(LanguageContext);
  if (!value) throw new Error('useI18n must be used within LanguageProvider');
  return value;
}
