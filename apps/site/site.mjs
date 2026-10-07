import { mountClawdGame } from './clawd-game.mjs';

const root = document.documentElement;
const themeButton = document.querySelector('#theme-toggle');
let theme = matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
try { const saved = localStorage.getItem('lintel.site.theme'); if (saved === 'light' || saved === 'dark') theme = saved; } catch {}
function applyTheme() {
  root.dataset.theme = theme;
  const label = theme === 'light' ? '切换到夜色' : '切换到日光';
  themeButton.setAttribute('aria-label', label);
  themeButton.title = label;
  themeButton.firstElementChild.textContent = theme === 'light' ? '☾' : '☼';
}
applyTheme();
themeButton.addEventListener('click', () => {
  theme = theme === 'light' ? 'dark' : 'light';
  applyTheme();
  try { localStorage.setItem('lintel.site.theme', theme); } catch {}
});

const features = {
  protect: {
    label: '保护方案', title: '减少不必要的外发', description: '选择你要改变的设置，其余保持原样。',
    rows: [['可选遥测', '保持原值 → 关闭'], ['错误报告', '保持原值 → 关闭'], ['你的其他设置', '保留']],
    note: '先看准确范围，再决定是否执行。',
    result: [['指定配置', '已写入并读回'], ['实际运行效果', '需要新启动后核对'], ['其他设置', '未改动']],
    resultNote: '配置已读回，不代表所有流量都已受约束。',
  },
  preserve: {
    label: '工作保全', title: '把心血，好好收起来', description: '选择具体原件，先核对范围，再生成加密工作包。',
    rows: [['选中的 3 份原件', '加入加密归档'], ['未选资料', '不加入'], ['原目录与登录', '保留']],
    note: '这是只归档的示例计划；原件选择不读取凭据或自动处理登录。',
    result: [['工作归档', '已生成并读回'], ['原目录与登录', '保留'], ['包内资料', '可阅读，不承诺续聊']],
    resultNote: '只归档不注销或清理。后续可独立读包、选片段整理可编辑交接稿；迁入另需预览批准。',
  },
  restore: {
    label: '记录与恢复', title: '每一次改动，都有来处', description: '保留任务回执，恢复前再核对当前值。',
    rows: [['原任务', '读取已记录的变更'], ['当前配置', '核对后续编辑'], ['恢复范围', '仅 Lintel 改动的字段']],
    note: '发现后续编辑时停止恢复，让你的新改动留下。',
    result: [['已匹配的字段', '按原值恢复'], ['其他设置', '未改动'], ['恢复结果', '已读回并记录']],
    resultNote: '这是没有冲突的示例。实际恢复需要单独预览、批准。',
  },
};
const tabs = [...document.querySelectorAll('[data-feature]')];
const panel = document.querySelector('#demo-panel');
const content = document.querySelector('#demo-content');
const next = document.querySelector('#demo-next');
let selected = 'protect', receipt = false;
function renderDemo() {
  const feature = features[selected];
  tabs.forEach(tab => {
    const active = tab.dataset.feature === selected;
    tab.setAttribute('aria-selected', String(active));
    tab.tabIndex = active ? 0 : -1;
    tab.querySelector('.tab-sign').textContent = active ? '−' : '+';
  });
  panel.setAttribute('aria-labelledby', `tab-${selected}`);
  content.innerHTML = `<p class="eyebrow">${feature.label}${receipt ? ' / 示例回执' : ''}</p><h3>${receipt ? '结果，也讲清楚。' : feature.title}</h3><p>${receipt ? '这里展示一份示例结果，不会执行真实操作。' : feature.description}</p><div class="demo-plan">${(receipt ? feature.result : feature.rows).map(([label, value]) => `<div><span>${label}</span><span class="${value.includes('保留') || value === '未改动' ? 'muted' : 'demo-change'}">${value}</span></div>`).join('')}</div><div class="demo-footnote">${receipt ? feature.resultNote : feature.note}</div>`;
  document.querySelector('#demo-step').textContent = receipt ? '02 / 示例回执' : '01 / 预览计划';
  next.textContent = receipt ? '返回示例计划' : '查看示例回执';
}
tabs.forEach((tab, index) => {
  tab.addEventListener('click', () => { selected = tab.dataset.feature; receipt = false; renderDemo(); });
  tab.addEventListener('keydown', event => {
    const offsets = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 };
    if (!(event.key in offsets) && !['Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const nextIndex = event.key === 'Home' ? 0 : event.key === 'End' ? tabs.length - 1 : (index + offsets[event.key] + tabs.length) % tabs.length;
    tabs[nextIndex].click(); tabs[nextIndex].focus();
  });
});
next.addEventListener('click', () => { receipt = !receipt; renderDemo(); });
const compact = matchMedia('(max-width: 700px)');
function orientTabs() { document.querySelector('[role=tablist]').setAttribute('aria-orientation', compact.matches ? 'horizontal' : 'vertical'); }
compact.addEventListener('change', orientTabs); orientTabs();
mountClawdGame(document.querySelector('#clawd-game'));
