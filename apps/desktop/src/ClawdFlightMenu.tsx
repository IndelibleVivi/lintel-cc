import { useEffect, useRef, useState, type ReactNode, type RefObject } from 'react';
import { Icon } from './ui';
import { useI18n } from './i18n-provider';
import './clawd-interactions.css';

export type ClawdDestination = 'landscapes' | 'runner';
const threshold = 76;

export default function ClawdFlightMenu({ scrollRef, onChoose, actions }: {
  scrollRef?: RefObject<HTMLElement | null>;
  onChoose?: (destination: ClawdDestination) => void;
  actions: ReactNode;
}) {
  const { t } = useI18n();
  const [pull, setPull] = useState(0);
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const first = useRef<HTMLDivElement>(null);
  const container = useRef<HTMLDivElement>(null);
  const distance = useRef(0);
  const returnFocus = useRef(false);
  function close() { returnFocus.current = true; setOpen(false); }

  useEffect(() => {
    if (open) first.current?.querySelector('button')?.focus({ preventScroll: true });
    else if (returnFocus.current) { trigger.current?.focus({ preventScroll: true }); returnFocus.current = false; }
  }, [open]);

  useEffect(() => {
    const scroller = scrollRef?.current;
    if (!scroller) return;
    let start: { x: number; y: number } | null = null;
    let wheelTimer: ReturnType<typeof setTimeout> | undefined;
    const ignored = (target: EventTarget | null) => target instanceof Element && !!target.closest('button, input, select, textarea, a, label, summary, p, h1, h2, h3, code, pre, [role="dialog"], .detail-panel');
    const eligible = (target: EventTarget | null) => !open && scroller.scrollTop <= 0 && !document.querySelector('dialog[open]') && !ignored(target);
    const update = (amount: number) => {
      if (amount > 4 && distance.current <= 4) window.getSelection()?.removeAllRanges();
      distance.current = Math.min(112, Math.max(0, amount)); setPull(distance.current);
    };
    const finish = (cancel = false) => {
      if (!cancel && distance.current >= threshold) setOpen(true);
      start = null; update(0);
    };
    const down = (event: PointerEvent) => {
      if (event.pointerType !== 'touch' && event.button === 0 && eligible(event.target)) start = { x: event.clientX, y: event.clientY };
    };
    const move = (event: PointerEvent) => {
      if (!start || event.pointerType === 'touch') return;
      if (Math.abs(event.clientX - start.x) > Math.max(18, Math.abs(event.clientY - start.y))) { finish(true); return; }
      update((event.clientY - start.y) * .6);
      if (distance.current > 3) event.preventDefault();
    };
    const up = () => { if (start) finish(); };
    const cancel = () => finish(true);
    const touchStart = (event: TouchEvent) => {
      if (event.touches.length === 1 && eligible(event.target)) start = { x: event.touches[0].clientX, y: event.touches[0].clientY };
      else finish(true);
    };
    const touchMove = (event: TouchEvent) => {
      if (!start || event.touches.length !== 1) return;
      const dx = event.touches[0].clientX - start.x;
      const dy = event.touches[0].clientY - start.y;
      if (dy < 0 || Math.abs(dx) > Math.max(12, dy)) { finish(true); return; }
      if (dy > 4) { event.preventDefault(); update(dy * .6); }
    };
    const wheel = (event: WheelEvent) => {
      if (!eligible(event.target) || event.deltaY >= 0 || event.ctrlKey || Math.abs(event.deltaX) > Math.abs(event.deltaY)) return;
      event.preventDefault();
      update(distance.current + Math.min(28, -event.deltaY * .45));
      clearTimeout(wheelTimer); wheelTimer = setTimeout(() => finish(), 180);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { cancel(); if (open) close(); }
    };
    scroller.addEventListener('pointerdown', down);
    window.addEventListener('pointermove', move, { passive: false });
    window.addEventListener('pointerup', up);
    window.addEventListener('pointercancel', cancel);
    scroller.addEventListener('touchstart', touchStart, { passive: true });
    scroller.addEventListener('touchmove', touchMove, { passive: false });
    scroller.addEventListener('touchend', up);
    scroller.addEventListener('touchcancel', cancel);
    scroller.addEventListener('wheel', wheel, { passive: false });
    window.addEventListener('keydown', escape);
    window.addEventListener('blur', cancel);
    return () => {
      clearTimeout(wheelTimer);
      scroller.removeEventListener('pointerdown', down); window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up); window.removeEventListener('pointercancel', cancel);
      scroller.removeEventListener('touchstart', touchStart); scroller.removeEventListener('touchmove', touchMove);
      scroller.removeEventListener('touchend', up); scroller.removeEventListener('touchcancel', cancel);
      scroller.removeEventListener('wheel', wheel); window.removeEventListener('keydown', escape);
      window.removeEventListener('blur', cancel);
    };
  }, [open, scrollRef]);

  useEffect(() => {
    if (!open) return;
    const dismiss = (event: PointerEvent) => { if (!container.current?.contains(event.target as Node)) setOpen(false); };
    window.addEventListener('pointerdown', dismiss);
    return () => window.removeEventListener('pointerdown', dismiss);
  }, [open]);

  function choose(destination: ClawdDestination) { trigger.current?.focus({ preventScroll: true }); setOpen(false); onChoose?.(destination); }
  return <div ref={container} className={`clawd-flight ${open ? 'is-open' : ''} ${pull > 0 ? 'is-pulling' : ''}`} style={{ '--pull': `${pull}px` } as React.CSSProperties} onBlur={event => { if (event.relatedTarget && !event.currentTarget.contains(event.relatedTarget)) setOpen(false); }}>
    <button ref={trigger} className="clawd-pull-trigger" aria-label={t('label.clawdCompanion')} aria-expanded={open} aria-controls="clawd-pocket" title={t('clawd.pullHint')} onClick={() => open ? close() : setOpen(true)}>
      <span>{t('clawd.choose')}</span><Icon name="chevron" size={11}/><span className="pull-progress" aria-hidden="true"/>
    </button>
    {pull > 0 && <span className="pull-hint" role="status">{pull >= threshold ? t('clawd.release') : t('clawd.pullMore')}</span>}
    {open && <section id="clawd-pocket" className="clawd-pocket" role="dialog" aria-label={t('clawd.pocketMenu')}>
      <div className="pocket-heading"><h2>{t('clawd.pocket')}</h2><button className="icon-button" aria-label={t('clawd.collapsePocket')} onClick={close}><Icon name="close" size={15}/></button></div>
      <div ref={first} className="pocket-pet-actions" aria-label={t('clawd.playActions')} onClick={event => { if ((event.target as Element).closest('button')) close(); }}>{actions}</div>
      {onChoose && <div className="pocket-destinations">
        <button onClick={() => choose('landscapes')}><span className="pocket-glyph" aria-hidden="true">☾</span><span>{t('clawd.goMoon')}</span><Icon name="arrow" size={15}/></button>
        <button onClick={() => choose('runner')}><span className="pocket-glyph" aria-hidden="true">↟</span><span>{t('clawd.jumpGame')}</span><Icon name="arrow" size={15}/></button>
      </div>}
      <p className="pocket-hint">{t('clawd.pocketHint')}</p>
    </section>}
  </div>;
}
