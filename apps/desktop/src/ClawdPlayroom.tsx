import { useEffect, useMemo, useRef, useState } from 'react';
import { mountClawdGame } from '../../site/clawd-game.mjs';
import '../../site/clawd-game.css';
import './clawd-playroom.css';
import { landscapes as plates, composeLandscape, LANDSCAPE_COLS, LANDSCAPE_ROWS } from './clawd-landscapes';
import { useI18n } from './i18n-provider';

// The album content is localized per plate index; the decorative ASCII grid and
// the shared runner stay independent of the App language.
function usePlate(index: number) {
  const { t } = useI18n();
  const p = plates[index];
  return { ...p, title: t(`album.${index}.title`), subtitle: t(`album.${index}.subtitle`), description: t(`album.${index}.description`), secret: t(`album.${index}.secret`) };
}

type Tab = 'landscapes' | 'runner';

function LandscapeAlbum() {
  const { t } = useI18n();
  const [index, setIndex] = useState(0);
  const [night, setNight] = useState(true);
  const [found, setFound] = useState<number[]>([]);
  const discovered = found.includes(index);
  const plate = usePlate(index);
  const cells = useMemo(() => composeLandscape(index, discovered), [index, discovered]);
  return <div className={`cp-album ${night ? 'cp-night' : 'cp-day'}`}>
    <div className="cp-plate-toolbar"><span className="cp-edition">{t('album.edition',{index:String(index + 1).padStart(2, '0'),place:plate.place})}</span><button onClick={() => setNight(value => !value)} aria-label={t(night ? 'album.toggleWarm' : 'album.toggleNight')}>{t(night ? 'album.dayLabel' : 'album.nightLabel')}</button></div>
    <figure className="cp-plate" aria-label={plate.description}>
      <div className="cp-art-scroll"><div className="cp-art-grid">
        <pre className="cp-ascii" aria-hidden="true">{cells.map((cell, i) => <span className={`cp-cell cp-ink-${cell.ink} ${cell.char === '█' ? 'cp-solid' : ''}`} key={i}>{cell.char}</span>)}</pre>
        <button className="cp-find-clawd" style={{ left: `${plate.clawd[0] / LANDSCAPE_COLS * 100}%`, top: `${plate.clawd[1] / LANDSCAPE_ROWS * 100}%` }} onClick={() => setFound(value => value.includes(index) ? value : [...value, index])} aria-label={t('album.findClawd',{title:plate.title})} title={t('album.titleHint')} />
      </div></div>
      <figcaption><div><span className="cp-plate-number">{t('album.plateNo',{index:String(index + 1).padStart(2, '0')})}</span><h3>{plate.title}</h3></div><p>{plate.subtitle}</p></figcaption>
    </figure>
    <div className="cp-album-footer"><p role="status">{discovered ? plate.secret : t('album.hint')}<span className="cp-star-count">{t('album.starCount',{found:found.length})}</span></p><div className="cp-pagination" aria-label={t('album.chooseScenery')}>{plates.map((item, i) => <button key={item.place} onClick={() => setIndex(i)} aria-pressed={index === i} aria-label={t('album.plateLabel',{index:i+1,title:t(`album.${i}.title`)})} title={t(`album.${i}.title`)}>{String(i + 1).padStart(2, '0')}{found.includes(i) && <span aria-label={t('album.foundStar')}>·</span>}</button>)}</div></div>
  </div>;
}

function Runner({ reducedMotion }: { reducedMotion: boolean }) {
  const { locale } = useI18n();
  const root = useRef<HTMLDivElement>(null);
  // The shared runner is mounted once; a language change relabels it live
  // (labels/aria/status) without disposing the run or losing the score/state.
  const game = useRef<ReturnType<typeof mountClawdGame> | null>(null);
  useEffect(() => {
    if (!root.current) return;
    const dispose = mountClawdGame(root.current, { scoreKey: 'lintel.clawd.runner.best', language: locale, reducedMotion });
    game.current = dispose;
    return () => { game.current = null; dispose(); };
  }, []);
  useEffect(() => { game.current?.setLanguage?.(locale); }, [locale]);
  useEffect(() => { game.current?.setReducedMotion?.(reducedMotion); }, [reducedMotion]);
  return <div ref={root}/>;
}

export default function ClawdPlayroom({ initialTab = 'landscapes', reducedMotion = false }: { initialTab?: Tab; reducedMotion?: boolean }) {
  const { t } = useI18n();
  const [tab, setTab] = useState<Tab>(initialTab);
  return <section className="clawd-playroom" aria-label={t('playroom.aria')}>
    <div className="cp-intro"><p>{t('playroom.intro')}</p><div className="cp-tabs" role="tablist" aria-label={t('playroom.tabsAria')} onKeyDown={event => { if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) { event.preventDefault(); const next = event.key === 'Home' ? 'landscapes' : event.key === 'End' ? 'runner' : tab === 'landscapes' ? 'runner' : 'landscapes'; setTab(next); event.currentTarget.querySelector<HTMLButtonElement>(`#cp-${next}-tab`)?.focus(); } }}><button role="tab" id="cp-landscapes-tab" tabIndex={tab === 'landscapes' ? 0 : -1} aria-selected={tab === 'landscapes'} aria-controls="cp-landscapes-panel" onClick={() => setTab('landscapes')}>{t('playroom.tabLandscapes')}</button><button role="tab" id="cp-runner-tab" tabIndex={tab === 'runner' ? 0 : -1} aria-selected={tab === 'runner'} aria-controls="cp-runner-panel" onClick={() => setTab('runner')}>{t('playroom.tabRunner')}</button></div></div>
    <div role="tabpanel" id="cp-landscapes-panel" aria-labelledby="cp-landscapes-tab" hidden={tab !== 'landscapes'}><LandscapeAlbum /></div>
    {tab === 'runner' && <div role="tabpanel" id="cp-runner-panel" aria-labelledby="cp-runner-tab"><Runner reducedMotion={reducedMotion}/></div>}
  </section>;
}
