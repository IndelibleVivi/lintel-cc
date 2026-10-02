import { useState } from 'react';

export type ClawdMood = 'hello' | 'plan' | 'pack' | 'rest' | 'done' | 'care' | 'work';
type Scene = 'idle' | 'heart' | 'tea' | 'flower' | 'book' | 'sleep' | 'party' | 'stars' | 'friend';
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
function Sprite({ sleeping = false }: { sleeping?: boolean }) {
  return <g className="clawd-body" fill="currentColor">
    <path d="M10 0H70V20H80V30H70V40H66V50H60V40H56V50H50V40H30V50H24V40H20V50H14V40H10V30H0V20H10Z"/>
    <g className="clawd-eyes" fill="#252320">{sleeping ? <><path d="M19 18H27V21H19ZM55 18H63V21H55Z"/></> : <><path d="M20 10H24V20H20ZM56 10H60V20H56Z"/></>}</g>
  </g>;
}
function Decorations({ scene }: { scene: Scene }) {
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
export default function Clawd({ mood = 'hello', small = false, interactive = false, caption }: { mood?: ClawdMood; small?: boolean; interactive?: boolean; caption?: string }) {
  const [visit, setVisit] = useState(-1);
  const surprise = visit >= 0 ? surprises[visit % surprises.length] : null;
  const scene = surprise?.scene ?? scenes[mood];
  const sprite = <svg className="clawd-scene" viewBox="0 0 120 80" shapeRendering="crispEdges" aria-hidden="true"><g transform="translate(20 22)"><Sprite sleeping={scene === 'sleep'}/></g><Decorations scene={scene}/></svg>;
  return <div className={`clawd ${small ? 'clawd-small' : ''} clawd-${mood} scene-${scene}`}>
    {interactive ? <button className="clawd-pet" aria-label="摸摸 Clawd，发现小彩蛋" onClick={() => setVisit(value => value + 1)} title="psst… 摸摸我">{sprite}</button> : sprite}
    {interactive && <span className={`clawd-caption ${surprise ? 'revealed' : ''}`} role="status">{surprise?.text ?? caption ?? 'psst… 摸摸我'}</span>}
    {!interactive && caption && <span className="clawd-caption revealed">{caption}</span>}
  </div>;
}
