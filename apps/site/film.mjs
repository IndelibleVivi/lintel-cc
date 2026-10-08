// Owns the opt-in short-film player for apps/site.
//
// Enhance the same-origin movie link into a native dark dialog; keep the no-JS link intact.
// The browser owns video controls, the same-origin poster and captions. No autoplay or storage;
// preload=none leaves the movie download until explicit play.
//
// Contract this module relies on (owned by index.html/styles.css):
//   #film-open        an <a class="film-open"> whose href is the film source;
//                     it becomes the dialog trigger.
//   #film-dialog      a native <dialog class="film-dialog"> holding:
//                       .film-close   one close button with an accessible label
//                       <video class="film-video" controls playsinline preload=none>
//
// It never reads or writes storage, clipboard, network beyond the chosen film, or any native
// bridge, and it removes every listener it adds when its disposer runs.

export function mountFilmPlayer(root = document) {
  const link = root.querySelector('#film-open');
  const dialog = root.querySelector('#film-dialog');
  if (!link || !dialog) return () => {};
  if (typeof dialog.showModal !== 'function') return () => {}; // leave the native link entrance intact

  const video = dialog.querySelector('video');
  const closeButton = dialog.querySelector('.film-close');
  let lastFocus = null;
  let disposed = false;

  // Reuse the trigger copy (the anchor is also the no-JS entrance) as the dialog's accessible name.
  const label = root.querySelector('#film-open-label');
  if (label) dialog.setAttribute('aria-label', label.textContent.trim());

  // The film is only fetched on explicit play (preload=none remains). If a
  // previous session paused mid-film, reopen from a clean state.
  const resetVideo = () => {
    if (!video) return;
    video.pause();
    if (video.currentTime) video.currentTime = 0;
  };

  // Keep the dialog's hidden state in sync synchronously on the open/close paths. Doing it here
  // (rather than only on the async `toggle` event) means a just-opened dialog is never left
  // aria-hidden="true" for assistive tech; the closed state stays inert per the initial markup.
  const reflectHidden = () => dialog.setAttribute('aria-hidden', String(!dialog.open));

  const open = event => {
    if (disposed) return;
    if (event) event.preventDefault();
    lastFocus = document.activeElement;
    resetVideo();
    dialog.showModal();
    reflectHidden();
    // Move focus into the dialog for keyboard users without stealing it from the native controls.
    if (closeButton) closeButton.focus({ preventScroll: true });
  };

  const close = () => {
    resetVideo();
    if (dialog.open) dialog.close();
    reflectHidden();
  };

  // Escape stops the film through the native `cancel` event; the `close` event owns the final
  // cleanup so button, Escape and programmatic dismissal all behave identically.
  const onCancel = event => { event.preventDefault(); close(); };
  const onClose = () => {
    if (dialog.open) return; // ignore a queued close event after a rapid reopen
    resetVideo();
    reflectHidden();
    // Restore initiation focus so reopening the film is one keystroke away.
    (lastFocus && lastFocus.isConnected ? lastFocus : link).focus({ preventScroll: true });
    lastFocus = null;
  };

  link.addEventListener('click', open);
  closeButton?.addEventListener('click', close);
  dialog.addEventListener('cancel', onCancel);
  dialog.addEventListener('close', onClose);
  // Reflect the native closed state for assistive technology too.
  reflectHidden();

  return () => {
    if (disposed) return;
    disposed = true;
    link.removeEventListener('click', open);
    closeButton?.removeEventListener('click', close);
    dialog.removeEventListener('cancel', onCancel);
    dialog.removeEventListener('close', onClose);
    resetVideo();
    if (dialog.open) dialog.close();
    reflectHidden();
  };
}
