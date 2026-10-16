export type MotionPreferences = { animations: boolean; logoBlink: boolean; terminalBlink: boolean };

const STORAGE_KEY = 'lintel.motion';

export function readMotionPreferences(): MotionPreferences {
  const defaults = { animations: true, logoBlink: true, terminalBlink: false };
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (!saved || saved.length > 256) return defaults;
    const value = JSON.parse(saved);
    if (!value || typeof value !== 'object' || Array.isArray(value)) return defaults;
    return {
      animations: typeof value.animations === 'boolean' ? value.animations : defaults.animations,
      logoBlink: typeof value.logoBlink === 'boolean' ? value.logoBlink : defaults.logoBlink,
      terminalBlink: typeof value.terminalBlink === 'boolean' ? value.terminalBlink : defaults.terminalBlink,
    };
  } catch { return defaults; }
}

export function applyMotionPreferences(preferences: MotionPreferences) {
  document.documentElement.dataset.motion = preferences.animations ? 'system' : 'off';
  document.documentElement.dataset.logoBlink = preferences.logoBlink ? 'on' : 'off';
  document.documentElement.dataset.terminalBlink = preferences.terminalBlink ? 'on' : 'off';
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences)); }
  catch { /* The in-memory choice still applies when storage is unavailable. */ }
}
