/** Mount the shared browser-only runner; dispose on React unmount or a replaced page. */
export function mountClawdGame(root: HTMLElement, options?: { scoreKey?: string }): () => void;
