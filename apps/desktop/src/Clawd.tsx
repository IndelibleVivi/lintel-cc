import { useEffect, useRef, useState, type CSSProperties, type PointerEvent, type RefObject } from 'react';
import './clawd-interactions.css';
import ClawdFlightMenu, { type ClawdDestination } from './ClawdFlightMenu';

export type ClawdMood = 'hello' | 'plan' | 'pack' | 'rest' | 'done' | 'care' | 'work';
type Scene = 'idle' | 'heart' | 'tea' | 'flower' | 'book' | 'sleep' | 'party' | 'stars' | 'friend' | 'fluster';
const surprises: { scene: Scene; text: string }[] = [
  { scene: 'heart', text: '摸摸收到。也给你一颗小小的心。' },
  { scene: 'tea', text: '给你留了一杯热的。' },
  { scene: 'flower', text: '今天也长出了一点点新东西。' },
  { scene: 'book', text: '你的工作，值得好好留下来。' },
  { scene: 'sleep', text: '嘘——小憩一下。草案还在。' },
  { scene: 'party', text: '没什么大事，也可以戴小帽子。' },
  { scene: 'stars', text: '把星星收进口袋，晚一点再用。' },
  { scene: 'friend', text: '叫来一个小伙伴，一起陪你。' },
];
const scenes: Record<ClawdMood, Scene> = { hello: 'idle', plan: 'book', pack: 'book', rest: 'sleep', done: 'stars', care: 'heart', work: 'tea' };

// Hand-drawn from the Clawd silhouette and its terminal appearance.
// Fixed geometry avoids block-character font fallback changing the eyes or feet.
// Visual reference provenance: docs/desktop.md. Clawd belongs to Anthropic.
function Sprite({ sleeping = false, flustered = false }: { sleeping?: boolean; flustered?: boolean }) {
  return <g className="clawd-body" fill="currentColor">
    <path d="M10 0H70V20H80V30H70V40H66V50H60V40H56V50H50V40H30V50H24V40H20V50H14V40H10V30H0V20H10Z"/>
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
  return null;
}
export default function Clawd({ mood = 'hello', small = false, interactive = false, caption, scrollRef, onChoose }: { mood?: ClawdMood; small?: boolean; interactive?: boolean; caption?: string; scrollRef?: RefObject<HTMLElement | null>; onChoose?: (destination: ClawdDestination) => void }) {
  const [visit, setVisit] = useState(-1);
  const [chosenScene, setChosenScene] = useState<Scene | null>(null);
  const [note, setNote] = useState('');
  const [held, setHeld] = useState(false);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const [look, setLook] = useState({ x: 0, y: 0 });
  const [jump, setJump] = useState(false);
  const [hiding, setHiding] = useState(false);
  const [flinging, setFlinging] = useState(false);
  const drag = useRef<{ x: number; y: number; moved: boolean; lastX: number; lastY: number; lastTime: number; vx: number; vy: number } | null>(null);
  const clicks = useRef({ time: 0, count: 0 });
  const ignoreClick = useRef(false);
  const longPress = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const jumpTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const flightTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => { clearTimeout(longPress.current); clearTimeout(jumpTimer.current); clearTimeout(flightTimer.current); }, []);
  const surprise = visit >= 0 ? surprises[visit % surprises.length] : null;
  const scene = chosenScene ?? surprise?.scene ?? scenes[mood];
  function react(next: Scene, text: string) { setChosenScene(next); setNote(text); }
  function comeBack() { clearTimeout(flightTimer.current); setHiding(false); setFlinging(false); setOffset({ x: 0, y: 0 }); clicks.current.count = 0; react('heart', '好吧，再给你摸一下。就一下。'); }
  function pet() {
    if (ignoreClick.current) { ignoreClick.current = false; return; }
    const now = performance.now();
    clicks.current.count = now - clicks.current.time < 460 ? clicks.current.count + 1 : 1;
    clicks.current.time = now;
    if (clicks.current.count >= 6) { setHiding(true); react('fluster', '……我躲好了。才没有露出脚脚。'); }
    else if (clicks.current.count >= 3) { react('fluster', clicks.current.count >= 5 ? '>< 再戳就要逃跑啦！' : '>< 呜哇，痒痒痒！'); }
    else { setChosenScene(null); setNote(''); setVisit(value => value + 1); }
  }
  function leap() {
    clearTimeout(jumpTimer.current); setJump(true); react('stars', '嘿咻！够到一颗星星。');
    jumpTimer.current = setTimeout(() => setJump(false), 700);
  }
  function start(event: PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    clearTimeout(flightTimer.current); setFlinging(false);
    drag.current = { x: event.clientX, y: event.clientY, moved: false, lastX: event.clientX, lastY: event.clientY, lastTime: performance.now(), vx: 0, vy: 0 }; ignoreClick.current = false; setHeld(true);
    clearTimeout(longPress.current);
    longPress.current = setTimeout(() => { ignoreClick.current = true; react('heart', '被你抱住了。再待一会儿。'); }, 520);
  }
  function move(event: PointerEvent<HTMLButtonElement>) {
    const bounds = event.currentTarget.getBoundingClientRect();
    setLook({ x: Math.max(-2, Math.min(2, (event.clientX - bounds.x - bounds.width / 2) / 16)), y: Math.max(-1, Math.min(1, (event.clientY - bounds.y - bounds.height / 2) / 20)) });
    if (!drag.current) return;
    const now = performance.now(), elapsed = Math.max(1, now - drag.current.lastTime);
    drag.current.vx = (event.clientX - drag.current.lastX) / elapsed;
    drag.current.vy = (event.clientY - drag.current.lastY) / elapsed;
    drag.current.lastX = event.clientX; drag.current.lastY = event.clientY; drag.current.lastTime = now;
    const x = event.clientX - drag.current.x, y = event.clientY - drag.current.y;
    if (Math.hypot(x, y) > 5) {
      drag.current.moved = true; clearTimeout(longPress.current);
      setOffset({ x: Math.max(-60, Math.min(80, x)), y: Math.max(-65, Math.min(45, y)) });
      react('heart', '哇——被抱起来啦。');
    }
  }
  function release(cancelled = false) {
    clearTimeout(longPress.current);
    const gesture = drag.current;
    if (!cancelled && gesture?.moved && performance.now() - gesture.lastTime < 120 && Math.hypot(gesture.vx, gesture.vy) > .65) {
      ignoreClick.current = true; drag.current = null; setHeld(false); setFlinging(true);
      setOffset({ x: Math.max(-125, Math.min(145, gesture.vx * 150)), y: Math.max(-95, Math.min(65, gesture.vy * 110)) });
      react('fluster', '呜——小螃蟹发射！');
      flightTimer.current = setTimeout(() => { setFlinging(false); setHiding(true); setOffset({ x: 0, y: 0 }); react('fluster', '降落完毕。让我躲一小会儿。'); }, 540);
      return;
    }
    if (drag.current?.moved) { ignoreClick.current = true; if (!cancelled) react('flower', '平稳着陆。送你一朵小花。'); }
    drag.current = null; setHeld(false); setOffset({ x: 0, y: 0 });
  }
  const sprite = <svg className="clawd-scene" viewBox="0 0 120 80" shapeRendering="crispEdges" aria-hidden="true"><g transform="translate(20 22)"><Sprite sleeping={scene === 'sleep'} flustered={scene === 'fluster'}/></g><Decorations scene={scene}/></svg>;
  return <div className={`clawd ${small ? 'clawd-small' : ''} clawd-${mood} scene-${scene} ${held ? 'is-held' : ''} ${jump ? 'is-jumping' : ''} ${hiding ? 'is-hiding' : ''} ${flinging ? 'is-flinging' : ''}`} style={{ '--pet-x': `${offset.x}px`, '--pet-y': `${offset.y}px`, '--look-x': `${look.x}px`, '--look-y': `${look.y}px` } as CSSProperties}>
    {interactive ? <><button className="clawd-pet" aria-label="摸摸 Clawd，发现小彩蛋" aria-describedby="clawd-pet-instructions" disabled={hiding} onPointerDown={start} onPointerMove={move} onPointerUp={() => release()} onPointerCancel={() => release(true)} onLostPointerCapture={() => { if (drag.current) release(true); }} onPointerLeave={() => setLook({ x: 0, y: 0 })} onClick={pet} onKeyDown={event => { if (event.key === 'ArrowUp') { event.preventDefault(); leap(); } if (event.key === 'Escape') { release(true); setChosenScene(null); setNote(''); setVisit(-1); } }} title="摸摸、抱起来，连着戳会害羞哦">{sprite}</button><span id="clawd-pet-instructions" className="sr-only">点按摸摸，快速连续点击会害羞躲藏，按住拥抱，拖动抱起来，快速甩动并松手弹飞，上方向键跳跃。躲藏后点露出的脚脚或哄回来。</span>{hiding && <button className="clawd-hideout" aria-label="找到 Clawd 露出的脚脚，哄它回来" onClick={comeBack}><svg width="54" height="30" viewBox="0 0 54 30" shapeRendering="crispEdges" aria-hidden="true"><path fill="currentColor" d="M21 17H25V26H21ZM29 17H33V26H29Z"/><path fill="var(--bg)" d="M3 0H50V19H3Z"/><path stroke="var(--line)" d="M2 19H51"/><path fill="currentColor" opacity=".35" d="M0 27H3V29H0ZM9 25H12V27H9Z"/></svg></button>}<ClawdFlightMenu scrollRef={scrollRef} onChoose={onChoose} actions={hiding ? <button onClick={comeBack}>哄回来</button> : <><button onClick={() => react('tea', '咕嘟。你也记得喝水。')}>喝茶</button><button onClick={leap}>跳跳</button><button onClick={() => scene === 'sleep' ? react('flower', '醒啦，我们继续。') : react('sleep', '在你旁边，睡一小会儿。')}>{scene === 'sleep' ? '叫醒' : '小憩'}</button></>}/></> : sprite}
    {interactive && <span className={`clawd-caption ${surprise || note ? 'revealed' : ''}`} role="status">{note || surprise?.text || caption || '摸摸我，也可以把我抱起来。'}</span>}
    {!interactive && caption && <span className="clawd-caption revealed">{caption}</span>}
  </div>;
}
