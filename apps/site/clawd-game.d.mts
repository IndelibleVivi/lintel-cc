/** Mount the shared browser-only runner; dispose on React unmount or a replaced page.
 *  `language` selects a finite copy table ('zh-CN' default, 'en'); the returned
 *  disposer also exposes `setLanguage(next)` for a live switch that preserves the run. */
export function mountClawdGame(root: HTMLElement, options?: { scoreKey?: string; language?: string }): (() => void) & { setLanguage?: (language: string) => void; language?: () => string };
