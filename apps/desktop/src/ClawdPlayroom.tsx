import { useEffect, useMemo, useRef, useState } from 'react';
import { mountClawdGame } from '../../site/clawd-game.mjs';
import '../../site/clawd-game.css';
import './clawd-playroom.css';
import { landscapes as plates, composeLandscape, LANDSCAPE_COLS, LANDSCAPE_ROWS } from './clawd-landscapes';

type Tab = 'landscapes' | 'runner';

function LandscapeAlbum() {
  const [index, setIndex] = useState(0);
  const [night, setNight] = useState(true);
  const [found, setFound] = useState<number[]>([]);
  const discovered = found.includes(index);
  const plate = plates[index];
  const cells = useMemo(() => composeLandscape(index, discovered), [index, discovered]);
  return <div className={`cp-album ${night ? 'cp-night' : 'cp-day'}`}>
    <div className="cp-plate-toolbar"><span className="cp-edition">{String(index + 1).padStart(2, '0')} / 04 · {plate.place}</span><button onClick={() => setNight(value => !value)} aria-label={night ? '切换暖纸风景' : '切换夜色风景'}>{night ? '☼ 暖纸' : '☾ 夜色'}</button></div>
    <figure className="cp-plate" aria-label={plate.description}>
      <div className="cp-art-scroll"><div className="cp-art-grid">
        <pre className="cp-ascii" aria-hidden="true">{cells.map((cell, i) => <span className={`cp-cell cp-ink-${cell.ink} ${cell.char === '█' ? 'cp-solid' : ''}`} key={i}>{cell.char}</span>)}</pre>
        <button className="cp-find-clawd" style={{ left: `${plate.clawd[0] / LANDSCAPE_COLS * 100}%`, top: `${plate.clawd[1] / LANDSCAPE_ROWS * 100}%` }} onClick={() => setFound(value => value.includes(index) ? value : [...value, index])} aria-label={`找到${plate.title}里的 Clawd，收一颗星`} title="psst… Clawd 在这里" />
      </div></div>
      <figcaption><div><span className="cp-plate-number">PLATE {String(index + 1).padStart(2, '0')}</span><h3>{plate.title}</h3></div><p>{plate.subtitle}</p></figcaption>
    </figure>
    <div className="cp-album-footer"><p role="status">{discovered ? plate.secret : '在画里找到橘色 Clawd，轻轻点一下。'}<span className="cp-star-count">✦ {found.length} / 4</span></p><div className="cp-pagination" aria-label="选择风景">{plates.map((item, i) => <button key={item.title} onClick={() => setIndex(i)} aria-pressed={index === i} aria-label={`第 ${i + 1} 幅：${item.title}`} title={item.title}>{String(i + 1).padStart(2, '0')}{found.includes(i) && <span aria-label="已找到星星">·</span>}</button>)}</div></div>
  </div>;
}

function Runner() {
  const root = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!root.current) return;
    return mountClawdGame(root.current, { scoreKey: 'lintel.clawd.runner.best' });
  }, []);
  return <div ref={root}/>;
}

export default function ClawdPlayroom({ initialTab = 'landscapes' }: { initialTab?: Tab }) {
  const [tab, setTab] = useState<Tab>(initialTab);
  return <section className="clawd-playroom" aria-label="Clawd 的小小游乐室">
    <div className="cp-intro"><p>把手头的事放一会儿。Clawd 带你去看看远处。</p><div className="cp-tabs" role="tablist" aria-label="游乐室内容" onKeyDown={event => { if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) { event.preventDefault(); const next = event.key === 'Home' ? 'landscapes' : event.key === 'End' ? 'runner' : tab === 'landscapes' ? 'runner' : 'landscapes'; setTab(next); event.currentTarget.querySelector<HTMLButtonElement>(`#cp-${next}-tab`)?.focus(); } }}><button role="tab" id="cp-landscapes-tab" tabIndex={tab === 'landscapes' ? 0 : -1} aria-selected={tab === 'landscapes'} aria-controls="cp-landscapes-panel" onClick={() => setTab('landscapes')}>点阵风景册</button><button role="tab" id="cp-runner-tab" tabIndex={tab === 'runner' ? 0 : -1} aria-selected={tab === 'runner'} aria-controls="cp-runner-panel" onClick={() => setTab('runner')}>跳一小段</button></div></div>
    <div role="tabpanel" id="cp-landscapes-panel" aria-labelledby="cp-landscapes-tab" hidden={tab !== 'landscapes'}><LandscapeAlbum /></div>
    {tab === 'runner' && <div role="tabpanel" id="cp-runner-panel" aria-labelledby="cp-runner-tab"><Runner /></div>}
  </section>;
}
