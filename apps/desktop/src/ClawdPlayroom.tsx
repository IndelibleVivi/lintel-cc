import { useEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import './clawd-playroom.css';
import { landscapes as plates, composeLandscape, LANDSCAPE_COLS, LANDSCAPE_ROWS } from './clawd-landscapes';

type Tab = 'landscapes' | 'runner';
const CLAWD_PATH = 'M10 0H70V20H80V30H70V40H66V50H60V40H56V50H50V40H30V50H24V40H20V50H14V40H10V30H0V20H10Z';

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

type GameStatus = 'ready' | 'running' | 'paused' | 'over';
type Obstacle = { x: number; width: number; height: number; kind: number };
type Run = { y: number; velocity: number; distance: number; speed: number; spawn: number; obstacles: Obstacle[]; status: GameStatus };
const GROUND = 204;
const PLAYER_X = 78;
const PLAYER_HEIGHT = 35;
const SCORE_KEY = 'lintel.clawd.runner.best';
function newRun(): Run { return { y: GROUND - PLAYER_HEIGHT, velocity: 0, distance: 0, speed: 225, spawn: 1.7, obstacles: [], status: 'ready' }; }
function readBest() {
  try { const value = Number(localStorage.getItem(SCORE_KEY)); return { score: Number.isFinite(value) && value > 0 ? Math.floor(value) : 0, stored: true }; }
  catch { return { score: 0, stored: false }; }
}

function Runner() {
  const canvas = useRef<HTMLCanvasElement>(null);
  const board = useRef<HTMLDivElement>(null);
  const run = useRef<Run>(newRun());
  const [status, setStatus] = useState<GameStatus>('ready');
  const [score, setScore] = useState(0);
  const [best, setBest] = useState(readBest);
  const bestRef = useRef(best);
  const draw = () => {
    const element = canvas.current, ctx = element?.getContext('2d');
    if (!element || !ctx) return;
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    if (element.width !== 720 * ratio) { element.width = 720 * ratio; element.height = 264 * ratio; }
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    const game = run.current;
    ctx.fillStyle = '#f5f0e5'; ctx.fillRect(0, 0, 720, 264);
    ctx.fillStyle = '#ded6c6';
    for (let x = 0; x < 720; x += 5) {
      const offset = (x + game.distance * .13) % 760;
      const hill = 149 - Math.sin(offset / 114) * 23 - Math.sin(offset / 61) * 11;
      for (let y = Math.round(hill); y < 195; y += 6) if ((x + y) % 3 === 0) ctx.fillRect(x, y, 1.4, 1.4);
    }
    ctx.fillStyle = '#b5aa92';
    const cloud = (x: number, y: number) => { for (let i = 0; i < 11; i++) ctx.fillRect(x + i * 5, y + (i < 3 || i > 8 ? 5 : 0), 3, 1); };
    cloud(150 - (game.distance * .07 % 870), 70); cloud(490 - (game.distance * .04 % 870), 47);
    ctx.strokeStyle = '#cab785'; ctx.lineWidth = 1;
    ctx.beginPath(); ctx.arc(628, 64, 17, 0, Math.PI * 2); ctx.stroke();
    ctx.strokeStyle = '#a69c88'; ctx.beginPath(); ctx.moveTo(0, GROUND + 1); ctx.lineTo(720, GROUND + 1); ctx.stroke();
    ctx.fillStyle = '#c5bbab';
    for (let i = 0; i < 30; i++) ctx.fillRect((i * 31 - game.distance * .8 % 31 + 720) % 720, GROUND + 9 + i % 3 * 6, i % 2 ? 5 : 2, 1);
    for (const obstacle of game.obstacles) {
      ctx.fillStyle = obstacle.kind === 0 ? '#9d9f80' : '#aa947c';
      if (obstacle.kind === 0) {
        ctx.fillRect(obstacle.x + 9, GROUND - obstacle.height, 9, obstacle.height);
        ctx.fillRect(obstacle.x, GROUND - obstacle.height + 13, 9, 7);
        ctx.fillRect(obstacle.x, GROUND - obstacle.height + 6, 5, 13);
        ctx.fillRect(obstacle.x + 18, GROUND - obstacle.height + 19, 8, 6);
        ctx.fillRect(obstacle.x + 22, GROUND - obstacle.height + 11, 4, 13);
      } else {
        ctx.fillRect(obstacle.x, GROUND - obstacle.height + 8, obstacle.width, obstacle.height - 8);
        ctx.fillRect(obstacle.x + 5, GROUND - obstacle.height, obstacle.width - 10, 8);
        ctx.fillStyle = '#f5f0e5'; ctx.fillRect(obstacle.x + 8, GROUND - obstacle.height + 8, 3, 9);
      }
    }
    ctx.save(); ctx.translate(PLAYER_X, game.y); ctx.scale(.7, .7);
    ctx.fillStyle = '#c17b60'; ctx.fill(new Path2D(CLAWD_PATH));
    ctx.fillStyle = '#312b25';
    if (game.status === 'over') { ctx.fillRect(19, 14, 8, 3); ctx.fillRect(55, 14, 8, 3); }
    else { ctx.fillRect(20, 10, 4, 10); ctx.fillRect(56, 10, 4, 10); }
    if (game.status === 'running' && game.y >= GROUND - PLAYER_HEIGHT && Math.floor(game.distance / 18) % 2) {
      ctx.fillStyle = '#f5f0e5'; ctx.fillRect(14, 44, 6, 6); ctx.fillRect(50, 44, 6, 6);
    }
    ctx.restore();
  };
  const changeStatus = (next: GameStatus) => { run.current.status = next; setStatus(next); };
  const saveBest = (notify = true) => {
    const currentScore = Math.floor(run.current.distance / 8);
    if (currentScore <= bestRef.current.score) return;
    let stored = true;
    try { localStorage.setItem(SCORE_KEY, String(currentScore)); } catch { stored = false; }
    bestRef.current = { score: currentScore, stored };
    if (notify) setBest(bestRef.current);
  };
  const pause = () => { if (run.current.status === 'running') { saveBest(); changeStatus('paused'); } };
  const start = () => {
    if (run.current.status !== 'paused') { run.current = newRun(); setScore(0); }
    changeStatus('running'); board.current?.focus();
  };
  const jump = () => {
    if (run.current.status === 'ready' || run.current.status === 'over') start();
    else if (run.current.status === 'paused') start();
    if (run.current.y >= GROUND - PLAYER_HEIGHT - .5) run.current.velocity = -525;
  };
  useEffect(() => {
    draw();
    if (status !== 'running') return;
    let frame = 0, last = 0, visibleScore = Math.floor(run.current.distance / 8);
    const step = (now: number) => {
      if (run.current.status !== 'running') return;
      const dt = last ? Math.min((now - last) / 1000, .04) : 0;
      last = now;
      const game = run.current;
      game.distance += game.speed * dt; game.speed = Math.min(440, 225 + game.distance / 130);
      game.velocity += 1450 * dt; game.y = Math.min(GROUND - PLAYER_HEIGHT, game.y + game.velocity * dt);
      if (game.y === GROUND - PLAYER_HEIGHT) game.velocity = 0;
      game.spawn -= dt;
      if (game.spawn <= 0) {
        const kind = Math.random() > .5 ? 1 : 0;
        game.obstacles.push({ x: 740, width: kind ? 30 : 26, height: kind ? 27 : 37, kind });
        game.spawn = 1.35 + Math.random() * .6;
      }
      for (const obstacle of game.obstacles) obstacle.x -= game.speed * dt;
      game.obstacles = game.obstacles.filter(obstacle => obstacle.x + obstacle.width > -10);
      const hit = game.obstacles.some(obstacle => PLAYER_X + 47 > obstacle.x + 3 && PLAYER_X + 9 < obstacle.x + obstacle.width - 3 && game.y + 28 > GROUND - obstacle.height + 3);
      const currentScore = Math.floor(game.distance / 8);
      if (visibleScore !== currentScore) { visibleScore = currentScore; setScore(currentScore); }
      if (hit) {
        game.status = 'over'; setStatus('over');
        saveBest();
      }
      draw();
      if (!hit) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [status]);
  useEffect(() => {
    const hidden = () => { if (document.hidden) pause(); };
    window.addEventListener('blur', pause); document.addEventListener('visibilitychange', hidden);
    return () => { window.removeEventListener('blur', pause); document.removeEventListener('visibilitychange', hidden); saveBest(false); };
  }, []);
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.target !== event.currentTarget) return;
    if ([' ', 'ArrowUp'].includes(event.key)) { event.preventDefault(); if (!event.repeat) jump(); }
    else if (event.key.toLowerCase() === 'p') { event.preventDefault(); if (!event.repeat) { if (run.current.status === 'running') pause(); else if (run.current.status === 'paused') start(); } }
  };
  const overlay = { ready: ['跑一小段，吹吹风', '前方有小石头与仙人掌。跳过去就好。'], paused: ['在这里歇一下', '路还在。准备好后继续出发。'], over: ['这次走到这里', `收下 ${score} 分，再跑一次也很好。`], running: ['', ''] }[status];
  return <div className="cp-runner" onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) pause(); }}>
    <div className="cp-runner-heading"><div><span className="cp-edition">CLAWD · LITTLE RUN</span><h3>小步向前</h3></div><div className="cp-scoreboard"><span>这次 <strong>{String(score).padStart(5, '0')}</strong></span><span>最高 <strong>{String(Math.max(best.score, score)).padStart(5, '0')}</strong></span></div></div>
    <div className="cp-game-board" ref={board} tabIndex={0} onKeyDown={keyDown} onPointerDown={event => { if ((event.target as HTMLElement).closest('button')) return; event.preventDefault(); board.current?.focus(); jump(); }} role="group" aria-label="Clawd 跳跃跑道" aria-describedby="cp-runner-instructions">
      <canvas ref={canvas} className="cp-game-canvas" width={720} height={264} aria-label="Clawd 沿暖纸色山间小路奔跑，跳过石头和仙人掌" />
      {status !== 'running' && <div className="cp-game-overlay"><h4>{overlay[0]}</h4><p>{overlay[1]}</p><button onClick={start}>{status === 'paused' ? '继续出发' : status === 'over' ? '再跑一次' : '开始跑步'} <span aria-hidden="true">→</span></button></div>}
    </div>
    <div className="cp-runner-controls"><button className="cp-jump" onClick={() => { jump(); board.current?.focus(); }} aria-label="跳跃；尚未开始时开始游戏">↥ 跳跃</button><button onClick={() => { if (status === 'running') pause(); else if (status === 'paused') start(); }} disabled={status === 'ready' || status === 'over'}>{status === 'paused' ? '继续' : '暂停'}</button><span role="status">{status === 'running' ? '路在慢慢加速。' : status === 'over' ? '碰到障碍了，再试一次。' : status === 'paused' ? '已暂停。' : '准备好了就出发。'}</span></div>
    <p id="cp-runner-instructions" className="cp-game-instructions">点一下跑道获取焦点，用 <kbd>空格</kbd> / <kbd>↑</kbd> 跳跃，<kbd>P</kbd> 暂停。也可以点跑道或「跳跃」。离开游戏或切换窗口会自动暂停。</p>
    <p className="cp-small-note">{best.stored ? '最高分只保存在这台设备，不发送任何数据。' : '本机存储不可用，最高分只保留到关闭游戏。'}风景静止，只有开始跑步后画面才会动。</p>
  </div>;
}

export default function ClawdPlayroom({ initialTab = 'landscapes' }: { initialTab?: Tab }) {
  const [tab, setTab] = useState<Tab>(initialTab);
  return <section className="clawd-playroom" aria-label="Clawd 的小小游乐室">
    <div className="cp-intro"><p>把手头的事放一会儿。Clawd 带你去看看远处。</p><div className="cp-tabs" role="tablist" aria-label="游乐室内容" onKeyDown={event => { if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) { event.preventDefault(); const next = event.key === 'Home' ? 'landscapes' : event.key === 'End' ? 'runner' : tab === 'landscapes' ? 'runner' : 'landscapes'; setTab(next); event.currentTarget.querySelector<HTMLButtonElement>(`#cp-${next}-tab`)?.focus(); } }}><button role="tab" id="cp-landscapes-tab" tabIndex={tab === 'landscapes' ? 0 : -1} aria-selected={tab === 'landscapes'} aria-controls="cp-landscapes-panel" onClick={() => setTab('landscapes')}>点阵风景册</button><button role="tab" id="cp-runner-tab" tabIndex={tab === 'runner' ? 0 : -1} aria-selected={tab === 'runner'} aria-controls="cp-runner-panel" onClick={() => setTab('runner')}>跳一小段</button></div></div>
    <div role="tabpanel" id="cp-landscapes-panel" aria-labelledby="cp-landscapes-tab" hidden={tab !== 'landscapes'}><LandscapeAlbum /></div>
    {tab === 'runner' && <div role="tabpanel" id="cp-runner-panel" aria-labelledby="cp-runner-tab"><Runner /></div>}
  </section>;
}
