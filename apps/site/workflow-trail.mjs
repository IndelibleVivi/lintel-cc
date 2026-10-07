// The four-step "work package journey" explainer for the website's #how-it-works section.
//
// This module owns only the interaction state of a host the main page already rendered. The host
// (#workflow-trail) ships hidden and holds four real buttons (data-trail-step="0".."3"), a
// "next step" button (data-trail-next) and a role="status" caption (data-trail-caption). Once the
// wire-up is complete the host is unhidden, so a reader without JavaScript simply never sees the
// control and still reads the plain four-step <ol>.
//
// Nothing here is an operation: it does not read the machine, make a plan, accept approval or
// execute anything. The caption copy is a fixed, explicitly synthetic explanation of what the real
// App/CLI flow would say; stepping never renders a fabricated success receipt or permission grant.
//
// No timers, requestAnimationFrame, storage, clipboard, network or native bridge. The main CSS owns
// every SVG/CSS transition for the picture keyed off host.dataset.step; a step switch stays usable
// with animations stopped and under prefers-reduced-motion. The disposer removes every listener.

const STEP_LABELS = ['选定范围', '查看计划', '批准执行', '核对结果'];

// One honest, explicitly synthetic sentence per step. Kept parallel to the four <li> in
// #how-it-works so the nearby interaction never contradicts the plain text.
const STEP_CAPTIONS = [
  '第 1 步：选好这次要处理的三份合成原件（CLAUDE.md、MEMORY.md、session.jsonl），以及它们所在的环境。这一步只确定范围，原件一直留在原处。',
  '第 2 步：查看这份计划——它冻结了范围、加密工作包的落点和将保留的内容。查看本身不改源原件，计划有自己的 ID。',
  '第 3 步：批准同一份计划后才会执行。批准把这次意图固定下来；如果条件发生变化，会停下并需要新的预览与批准。',
  '第 4 步：核对原任务。回执区分已接收与已完成，断开后按原 ID 回查、不重新提交。工作包可脱离原任务独立阅读，原件始终保留。',
];

// The next button advances 0→1→2→3 and then loops 3→0, relabelling on the last step.
const TRAIL_HINT = '合成说明：这里只展示流程，不读取电脑或执行操作。';

/**
 * Wire up the four-step workflow explainer.
 * @param {Element|null|undefined} host the #workflow-trail container
 * @returns {() => void} disposer that removes every listener this call added
 */
export function mountWorkflowTrail(host) {
  if (!host) return () => {};

  const buttons = [...host.querySelectorAll('[data-trail-step]')];

  // Without the full four-button contract there is nothing safe to drive; leave the host as
  // shipped (hidden) and hand back an inert disposer.
  if (buttons.length !== 4) return () => {};

  const steps = buttons
    .map(node => ({node, index:Number(node.dataset.trailStep)}))
    .filter(entry => Number.isInteger(entry.index))
    .sort((a, b) => a.index - b.index);
  if (steps.length !== 4) return () => {};

  const nextButton = host.querySelector('[data-trail-next]');
  const caption = host.querySelector('[data-trail-caption]');
  const total = steps.length;
  let current = 0;

  // The nearby <li> elements in #how-it-works. Optional: the module still works when the host is
  // mounted elsewhere (the coordinator's integration may differ); it just skips the highlight.
  const section = host.closest('#how-it-works') || document;
  const workflowItems = [...section.querySelectorAll('.workflow > li')];

  const applyHighlight = () => {
    workflowItems.forEach((li, i) => { if (i === current) li.dataset.highlight = 'true'; else delete li.dataset.highlight; });
  };

  // Single place that reconciles every outward surface from `current`. Idempotent, so rapid
  // reselection always converges on the latest state (no queued intermediate render).
  const render = () => {
    host.dataset.step = String(current);
    steps.forEach(entry => entry.node.setAttribute('aria-pressed', String(entry.index === current)));
    if (caption) caption.textContent = `${STEP_CAPTIONS[current]} ${TRAIL_HINT}`;
    if (nextButton) {
      const last = current === total - 1;
      nextButton.textContent = last ? '回到第 1 步 ↺' : `下一步：${STEP_LABELS[current + 1]} →`;
      nextButton.setAttribute('aria-label', last ? '回到第一步' : `前往第 ${current + 2} 步：${STEP_LABELS[current + 1]}`);
    }
    applyHighlight();
  };

  const selectStep = index => {
    if (!Number.isInteger(index) || index < 0 || index >= total || index === current) return;
    current = index;
    render();
  };

  const onStepClick = event => selectStep(Number(event.currentTarget.dataset.trailStep));
  const onNextClick = () => {
    current = current === total - 1 ? 0 : current + 1;
    render();
  };

  for (const {node} of steps) node.addEventListener('click', onStepClick);
  if (nextButton) nextButton.addEventListener('click', onNextClick);

  render();
  host.hidden = false; // fully initialised: reveal the control now

  return () => {
    for (const {node} of steps) node.removeEventListener('click', onStepClick);
    if (nextButton) nextButton.removeEventListener('click', onNextClick);
    workflowItems.forEach(li => delete li.dataset.highlight);
    host.hidden = true;
    delete host.dataset.step;
  };
}
