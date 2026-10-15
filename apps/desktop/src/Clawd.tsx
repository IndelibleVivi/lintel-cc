import { useEffect, useRef, useState, type CSSProperties, type PointerEvent, type RefObject } from 'react';
import './clawd-interactions.css';
import ClawdFlightMenu, { type ClawdDestination } from './ClawdFlightMenu';
import { useI18n } from './i18n-provider';

export type ClawdMood = 'hello' | 'plan' | 'pack' | 'rest' | 'done' | 'care' | 'work';
type Scene = 'idle' | 'heart' | 'tea' | 'flower' | 'book' | 'sleep' | 'party' | 'stars' | 'friend' | 'fluster' | 'moon' | 'snack';
const surprises: { scene: Scene; key: string }[] = [
  { scene: 'heart', key: 'clawd.surprise.heart' },
  { scene: 'tea', key: 'clawd.surprise.tea' },
  { scene: 'flower', key: 'clawd.surprise.flower' },
  { scene: 'book', key: 'clawd.surprise.book' },
  { scene: 'sleep', key: 'clawd.surprise.sleep' },
  { scene: 'party', key: 'clawd.surprise.party' },
  { scene: 'stars', key: 'clawd.surprise.stars' },
  { scene: 'friend', key: 'clawd.surprise.friend' },
  { scene: 'moon', key: 'clawd.surprise.moon' },
  { scene: 'snack', key: 'clawd.surprise.snack' },
];
const scenes: Record<ClawdMood, Scene> = { hello: 'idle', plan: 'book', pack: 'book', rest: 'sleep', done: 'stars', care: 'heart', work: 'tea' };
// Movement past this many pixels counts as a drag instead of a tap.
const dragThreshold = 6;
// Room kept between Clawd and the home edges; the top clears its caption bubble.
const dragPad = { x: 12, top: 54, bottom: 14 };
const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));

// Hand-drawn from the Clawd silhouette and its terminal appearance.
// Fixed geometry avoids block-character font fallback changing the eyes or feet.
// Visual reference provenance: docs/desktop.md. Clawd belongs to Anthropic.
function Sprite({ sleeping = false, flustered = false }: { sleeping?: boolean; flustered?: boolean }) {
  return <g className="clawd-body" fill="currentColor">
    <path d="M10 0H70V20H80V30H70V40H66V46H60V40H56V46H50V40H30V46H24V40H20V46H14V40H10V30H0V20H10Z"/>
    <g className="clawd-eyes" fill="#252320">{flustered ? <path d="M18 9H21V12H24V15H27V18H24V21H21V24H18V20H21V17H18ZM62 9H59V12H56V15H53V18H56V21H59V24H62V20H59V17H62Z"/> : sleeping ? <path d="M19 18H27V21H19ZM55 18H63V21H55Z"/> : <path d="M20 10H24V20H20ZM56 10H60V20H56Z"/>}</g>
    {flustered && <path d="M13 25H24V28H13ZM56 25H67V28H56Z" fill="#A35748" opacity=".7"/>}
  </g>;
}
function Decorations({ scene }: { scene: Scene }) {
  if (scene === 'fluster') return <g fill="#C27670"><path d="M13 16H16V23H13ZM7 23H10V26H7ZM104 17H107V24H104ZM110 26H113V29H110Z"/><text x="55" y="12" className="clawd-letters">!</text></g>;
  if (scene === 'heart') return <g className="clawd-float" fill="#C27670"><path d="M54 4H58V8H62V4H66V8H70V12H66V16H62V20H58V16H54V12H50V8H54Z"/></g>;
  if (scene === 'party') return <g fill="#A191BE"><path d="M50 18H70V14H66V10H62V6H58V10H54V14H50Z"/><path d="M58 0H62V4H58Z" fill="#D8AC59"/></g>;
  if (scene === 'stars') return <g className="clawd-float" fill="#B59856"><path d="M16 10H20V14H24V18H20V22H16V18H12V14H16ZM99 0H102V4H106V7H102V11H99V7H95V4H99ZM108 33H111V36H108Z"/></g>;
  if (scene === 'sleep') return <text x="91" y="18" className="clawd-letters">z z</text>;
  if (scene === 'tea') return <g><path fill="#AAA38B" d="M94 49H110V53H116V63H110V69H94Z"/><path fill="var(--bg)" d="M110 56H113V60H110Z"/><path fill="#CBB38C" d="M97 46H107V49H97Z"/><path className="clawd-steam" stroke="#AAA38B" strokeWidth="2" fill="none" d="M98 40V36H101V32M105 43V38H108V34"/></g>;
  if (scene === 'flower') return <g><path stroke="#858F68" strokeWidth="3" d="M108 50V69M108 61L102 57"/><path fill="#C88978" d="M105 37H111V41H115V47H111V51H105V47H101V41H105Z"/><path fill="#E1C98D" d="M105 41H111V47H105Z"/></g>;
  if (scene === 'book') return <g><path fill="#A6A48B" d="M91 51H101L104 54L107 51H117V69H107L104 72L101 69H91Z"/><path stroke="var(--bg)" strokeWidth="2" d="M104 56V67M94 56H100M108 56H114M94 60H100M108 60H114"/></g>;
  if (scene === 'friend') return <g transform="translate(97 53) scale(.3)"><Sprite/></g>;
  if (scene === 'moon') return <g className="clawd-float"><path fill="#E8D9AE" d="M54 2H62V6H66V14H62V18H54V14H50V6H54Z"/><path fill="#C9B787" d="M32 8H35V11H32ZM78 4H81V7H78Z"/></g>;
  if (scene === 'snack') return <g className="clawd-float"><path fill="#D8AC59" d="M96 50H108V54H112V62H108V66H96V62H92V54H96Z"/><path fill="#8B6C32" d="M98 54H101V57H98ZM105 58H108V61H105ZM98 61H101V64H98Z"/></g>;
  return null;
}
export default function Clawd({ mood = 'hello', small = false, interactive = false, caption, scrollRef, onChoose }: { mood?: ClawdMood; small?: boolean; interactive?: boolean; caption?: string; scrollRef?: RefObject<HTMLElement | null>; onChoose?: (destination: ClawdDestination) => void }) {
  const { t } = useI18n();
  const [visit, setVisit] = useState(-1);
  const [chosenScene, setChosenScene] = useState<Scene | null>(null);
  // Store the translation KEY, not the translated text, so a language switch
  // relabels the current Clawd note.
  const [noteKey, setNoteKey] = useState('');
  const [held, setHeld] = useState(false);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const [look, setLook] = useState({ x: 0, y: 0 });
  const [jump, setJump] = useState(false);
  const [hiding, setHiding] = useState(false);
  const [flinging, setFlinging] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [landing, setLanding] = useState(false);
  const drag = useRef<{ x: number; y: number; moved: boolean; lastX: number; lastY: number; lastTime: number; vx: number; vy: number; baseX: number; baseY: number; minX: number; maxX: number; minY: number; maxY: number } | null>(null);
  const clicks = useRef({ time: 0, count: 0 });
  const ignoreClick = useRef(false);
  const longPress = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const jumpTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const flightTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const landTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => { clearTimeout(longPress.current); clearTimeout(jumpTimer.current); clearTimeout(flightTimer.current); clearTimeout(landTimer.current); }, []);
  const surprise = visit >= 0 ? surprises[visit % surprises.length] : null;
  const scene = chosenScene ?? surprise?.scene ?? scenes[mood];
  function react(next: Scene, key: string) { setChosenScene(next); setNoteKey(key); }
  function comeBack() { clearTimeout(flightTimer.current); setHiding(false); setFlinging(false); setOffset({ x: 0, y: 0 }); clicks.current.count = 0; react('heart', 'clawd.okPetAgain'); }
  function pet() {
    if (ignoreClick.current) { ignoreClick.current = false; return; }
    const now = performance.now();
    clicks.current.count = now - clicks.current.time < 460 ? clicks.current.count + 1 : 1;
    clicks.current.time = now;
    if (clicks.current.count >= 6) { setHiding(true); react('fluster', 'clawd.hidFluster'); }
    else if (clicks.current.count >= 3) { react('fluster', clicks.current.count >= 5 ? 'clawd.tickle5' : 'clawd.tickle3'); }
    else { setChosenScene(null); setNoteKey(''); setVisit(value => value + 1); }
  }
  function leap() {
    clearTimeout(jumpTimer.current); setJump(true); react('stars', 'clawd.reachStar');
    jumpTimer.current = setTimeout(() => setJump(false), 700);
  }
  function start(event: PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0) return;
    const button = event.currentTarget;
    button.setPointerCapture(event.pointerId);
    clearTimeout(flightTimer.current); clearTimeout(landTimer.current);
    setFlinging(false); setLanding(false);
    // Freeze the reachable drag delta from the live home scroller so Clawd can
    // never leave the main content area or hide behind the top edge. The delta
    // bounds are measured against the button rect at pointer-down (which
    // already includes the current offset).
    const frame = (scrollRef?.current ?? button.closest('.main-content'))?.getBoundingClientRect();
    const self = button.getBoundingClientRect();
    const minX = frame ? frame.left + dragPad.x - self.left : 0, maxX = frame ? frame.right - dragPad.x - self.right : 0;
    const minY = frame ? frame.top + dragPad.top - self.top : 0, maxY = frame ? frame.bottom - dragPad.bottom - self.bottom : 0;
    drag.current = { x: event.clientX, y: event.clientY, moved: false, lastX: event.clientX, lastY: event.clientY, lastTime: performance.now(), vx: 0, vy: 0, baseX: offset.x, baseY: offset.y, minX: Math.min(minX, maxX), maxX: Math.max(minX, maxX), minY: Math.min(minY, maxY), maxY: Math.max(minY, maxY) };
    ignoreClick.current = false; setHeld(true);
    clearTimeout(longPress.current);
    longPress.current = setTimeout(() => { ignoreClick.current = true; react('heart', 'clawd.thanksHug'); }, 520);
  }
  function move(event: PointerEvent<HTMLButtonElement>) {
    const bounds = event.currentTarget.getBoundingClientRect();
    setLook({ x: Math.max(-2, Math.min(2, (event.clientX - bounds.x - bounds.width / 2) / 16)), y: Math.max(-1, Math.min(1, (event.clientY - bounds.y - bounds.height / 2) / 20)) });
    const gesture = drag.current;
    if (!gesture) return;
    const now = performance.now(), elapsed = Math.max(1, now - gesture.lastTime);
    gesture.vx = (event.clientX - gesture.lastX) / elapsed;
    gesture.vy = (event.clientY - gesture.lastY) / elapsed;
    gesture.lastX = event.clientX; gesture.lastY = event.clientY; gesture.lastTime = now;
    const dx = event.clientX - gesture.x, dy = event.clientY - gesture.y;
    if (!gesture.moved && Math.hypot(dx, dy) > dragThreshold) {
      gesture.moved = true; clearTimeout(longPress.current);
      setDragging(true); react('heart', 'clawd.pickedUp');
    }
    if (!gesture.moved) return;
    // Keep the drag from starting a page scroll or a text selection.
    event.preventDefault();
    setOffset({ x: gesture.baseX + clamp(dx, gesture.minX, gesture.maxX), y: gesture.baseY + clamp(dy, gesture.minY, gesture.maxY) });
  }
  function release(cancelled = false) {
    clearTimeout(longPress.current);
    const gesture = drag.current;
    drag.current = null; setDragging(false);
    if (!gesture) { setHeld(false); return; }
    if (!cancelled && gesture.moved && performance.now() - gesture.lastTime < 120 && Math.hypot(gesture.vx, gesture.vy) > .65) {
      ignoreClick.current = true; setHeld(false); setFlinging(true);
      setOffset({ x: gesture.baseX + clamp(gesture.vx * 150, gesture.minX, gesture.maxX), y: gesture.baseY + clamp(gesture.vy * 110, gesture.minY, gesture.maxY) });
      react('fluster', 'clawd.launch');
      flightTimer.current = setTimeout(() => { setFlinging(false); setHiding(true); setOffset({ x: 0, y: 0 }); react('fluster', 'clawd.landed'); }, 540);
      return;
    }
    // A plain drag leaves Clawd where it was dropped and settles it upright.
    if (gesture.moved) {
      ignoreClick.current = true;
      if (!cancelled) { react('flower', 'clawd.stableLand'); setLanding(true); landTimer.current = setTimeout(() => setLanding(false), 340); }
    }
    setHeld(false);
  }
  const sprite = <svg className="clawd-scene" viewBox="0 0 120 80" shapeRendering="crispEdges" aria-hidden="true"><g transform="translate(20 26)"><Sprite sleeping={scene === 'sleep'} flustered={scene === 'fluster'}/></g><Decorations scene={scene}/></svg>;
  return <div className={`clawd ${small ? 'clawd-small' : ''} clawd-${mood} scene-${scene} ${held ? 'is-held' : ''} ${dragging ? 'is-dragging' : ''} ${landing ? 'is-landing' : ''} ${jump ? 'is-jumping' : ''} ${hiding ? 'is-hiding' : ''} ${flinging ? 'is-flinging' : ''}`} style={{ '--pet-x': `${offset.x}px`, '--pet-y': `${offset.y}px`, '--look-x': `${look.x}px`, '--look-y': `${look.y}px` } as CSSProperties}>
    {interactive ? <><button className="clawd-pet" aria-label={t('clawd.pet')} aria-describedby="clawd-pet-instructions" disabled={hiding} onPointerDown={start} onPointerMove={move} onPointerUp={() => release()} onPointerCancel={() => release(true)} onLostPointerCapture={() => { if (drag.current) release(true); }} onPointerLeave={() => setLook({ x: 0, y: 0 })} onClick={pet} onKeyDown={event => { if (event.key === 'ArrowUp') { event.preventDefault(); leap(); } if (event.key === 'Escape') { release(true); setChosenScene(null); setNoteKey(''); setVisit(-1); } }} title={t('clawd.petTitle')}>{sprite}</button><span id="clawd-pet-instructions" className="sr-only">{t('clawd.petInstructions')}</span>{hiding && <button className="clawd-hideout" aria-label={t('clawd.hideout')} onClick={comeBack}><svg width="54" height="30" viewBox="0 0 54 30" shapeRendering="crispEdges" aria-hidden="true"><path fill="currentColor" d="M21 17H25V24H21ZM29 17H33V24H29Z"/><path fill="var(--bg)" d="M3 0H50V19H3Z"/><path stroke="var(--line)" d="M2 19H51"/><path fill="currentColor" opacity=".35" d="M0 27H3V29H0ZM9 25H12V27H9Z"/></svg></button>}<ClawdFlightMenu scrollRef={scrollRef} onChoose={onChoose} actions={hiding ? <button onClick={comeBack}>{t('clawd.comeBack')}</button> : <><button onClick={() => react('tea', 'clawd.sip')}>{t('clawd.tea')}</button><button onClick={leap}>{t('clawd.jump')}</button><button onClick={() => react('moon', 'clawd.reachedMoon')}>{t('clawd.moon')}</button><button onClick={() => scene === 'sleep' ? react('flower', 'clawd.goodmorning') : react('sleep', 'clawd.goodnight')}>{t(scene === 'sleep' ? 'clawd.wake' : 'clawd.nap')}</button></>}/></> : sprite}
    {interactive && <span className={`clawd-caption ${surprise || noteKey ? 'revealed' : ''}`} role="status">{noteKey ? t(noteKey) : surprise ? t(surprise.key) : caption || t('clawd.captionDefault')}</span>}
    {!interactive && caption && <span className="clawd-caption revealed">{caption}</span>}
  </div>;
}
