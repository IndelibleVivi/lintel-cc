// Finite route and guide map for the standalone website.
//
// Chinese templates own page structure; site-copy.mjs owns finite paired copy. prepare-site-languages.mjs
// projects finite paired copy and documentation bodies into the English mirror.
// Guide bodies are never copied here: each route points at one repository file.

export const SITE_ORIGIN = 'https://lintel.page';
export const REPOSITORY = 'https://github.com/IndelibleVivi/lintel-cc';
export const REPOSITORY_BLOB = `${REPOSITORY}/blob/main`;

export const GUIDE_ARTICLES = Object.freeze([
  Object.freeze({
    key: 'start-here',
    path: '/guide/start-here/',
    doc: 'start-here',
    group: 'start',
    title: Object.freeze({ zh: '第一次使用', en: 'First use' }),
    summary: Object.freeze({
      zh: '从一份合成资料开始，完成一次加密归档并重新打开它。',
      en: 'Start with one synthetic package: archive it, approve the task and read it again.',
    }),
  }),
  Object.freeze({
    key: 'quickstart',
    path: '/guide/quickstart/',
    doc: 'quickstart',
    group: 'start',
    title: Object.freeze({ zh: 'CLI 完整试用', en: 'Complete CLI trial' }),
    summary: Object.freeze({
      zh: '用一次性合成目录核对登记、预览、批准、原任务与独立读包。',
      en: 'Use a disposable synthetic root to check registration, preview, approval, original tasks and independent reading.',
    }),
  }),
  Object.freeze({
    key: 'concepts',
    path: '/guide/concepts/',
    doc: 'concepts',
    group: 'start',
    title: Object.freeze({ zh: '基本概念', en: 'Core concepts' }),
    summary: Object.freeze({
      zh: '分清主机、配置根、项目目录、冻结计划、任务与工作包。',
      en: 'Separate hosts, configuration roots, project directories, frozen plans, tasks and work packages.',
    }),
  }),
  Object.freeze({
    key: 'troubleshooting',
    path: '/guide/troubleshooting/',
    doc: 'troubleshooting',
    group: 'start',
    title: Object.freeze({ zh: '排错指南', en: 'Troubleshooting' }),
    summary: Object.freeze({
      zh: '从看到的结果开始，保留原 ID，先核对再决定下一步。',
      en: 'Start from the observed result, retain the original ID, and verify before choosing the next step.',
    }),
  }),
  Object.freeze({
    key: 'operator-guide',
    path: '/guide/operator-guide/',
    doc: 'operator-guide',
    group: 'tasks',
    title: Object.freeze({ zh: '操作指南', en: 'Operator guide' }),
    summary: Object.freeze({
      zh: '按六类实际任务进入设置、保全、清理、浏览器、SSH 与恢复。',
      en: 'Follow six practical tasks through settings, preservation, cleanup, browser, SSH and recovery.',
    }),
  }),
  Object.freeze({
    key: 'agents',
    path: '/guide/agents/',
    doc: 'agents',
    group: 'tasks',
    title: Object.freeze({ zh: 'Agent CLI', en: 'Agent CLI' }),
    summary: Object.freeze({
      zh: '独立 CLI、接口发现、秘密 stdin、portable work 与候选安装。',
      en: 'Standalone CLI, interface discovery, secret stdin, portable work and candidate installation.',
    }),
  }),
  Object.freeze({
    key: 'browser',
    path: '/guide/browser/',
    doc: 'browser',
    group: 'tasks',
    title: Object.freeze({ zh: '浏览器模块', en: 'Browser' }),
    summary: Object.freeze({
      zh: '扩展准备、profile 配对、有限站点操作与完整重启清理。',
      en: 'Extension preparation, profile pairing, finite site actions and cleanup after a full restart.',
    }),
  }),
  Object.freeze({
    key: 'remote',
    path: '/guide/remote/',
    doc: 'remote',
    group: 'tasks',
    title: Object.freeze({ zh: 'SSH 与远端', en: 'SSH and remote hosts' }),
    summary: Object.freeze({
      zh: '登记 alias、准备 runner、断线后按原任务 ID 查询。',
      en: 'Register an alias, prepare a runner, and query the original task after a disconnect.',
    }),
  }),
  Object.freeze({
    key: 'desktop',
    path: '/guide/desktop/',
    doc: 'desktop',
    group: 'tasks',
    title: Object.freeze({ zh: '桌面应用', en: 'Desktop app' }),
    summary: Object.freeze({
      zh: 'App 页面、语言、目标栏、会话资料、外观与本地构建。',
      en: 'App pages, language, target bar, sessions, appearance and local builds.',
    }),
  }),
  Object.freeze({
    key: 'network',
    path: '/guide/network/',
    doc: 'network',
    group: 'tasks',
    title: Object.freeze({ zh: '网络与出口', en: 'Network and egress' }),
    summary: Object.freeze({
      zh: '区分受控通道、IPv4／IPv6 实测、配置读回与强制约束。',
      en: 'Separate the controlled channel, IPv4/IPv6 tests, configuration readback and enforcement.',
    }),
  }),
  Object.freeze({
    key: 'services',
    path: '/guide/services/',
    doc: 'services',
    group: 'tasks',
    title: Object.freeze({ zh: 'systemd 服务', en: 'systemd services' }),
    summary: Object.freeze({
      zh: '检查、暂停与恢复一个明确绑定配置根的服务。',
      en: 'Inspect, pause and restore one service explicitly bound to a configuration root.',
    }),
  }),
  Object.freeze({
    key: 'telemetry',
    path: '/guide/telemetry/',
    doc: 'telemetry',
    group: 'tasks',
    title: Object.freeze({ zh: '遥测目标', en: 'Telemetry targets' }),
    summary: Object.freeze({
      zh: '区分静态目录、配置规则、受控测试与普通观察。',
      en: 'Separate the static catalog, configured rules, controlled tests and ordinary observations.',
    }),
  }),
  Object.freeze({
    key: 'app-updates',
    path: '/guide/app-updates/',
    doc: 'app-updates',
    group: 'tasks',
    title: Object.freeze({ zh: '安装与更新', en: 'Install and update' }),
    summary: Object.freeze({
      zh: 'App 更新、候选首装验收与发行记录准备的真实顺序。',
      en: 'The real order for App updates, candidate install acceptance and release-record preparation.',
    }),
  }),
  Object.freeze({
    key: 'core',
    path: '/guide/core/',
    doc: 'core',
    group: 'engineering',
    title: Object.freeze({ zh: 'Core 与 CLI', en: 'Core and CLI' }),
    summary: Object.freeze({
      zh: '计划、批准、执行、任务记录、工作包与恢复的共享实现。',
      en: 'The shared implementation for plans, approval, execution, records, packages and restoration.',
    }),
  }),
  Object.freeze({
    key: 'architecture',
    path: '/guide/architecture/',
    doc: 'architecture',
    group: 'engineering',
    title: Object.freeze({ zh: '架构与源码地图', en: 'Architecture and source map' }),
    summary: Object.freeze({
      zh: '四条执行路径、状态归属与实现入口。',
      en: 'Four execution paths, state ownership and implementation entry points.',
    }),
  }),
]);

export const SITE_ROUTES = Object.freeze([
  Object.freeze({ key: 'home', path: '/', source: 'index.html', enSource: 'en/index.html' }),
  Object.freeze({ key: 'download', path: '/download/', source: 'download.html' }),
  Object.freeze({ key: 'guide', path: '/guide/', source: 'guide.html' }),
  Object.freeze({ key: 'inside', path: '/inside/', source: 'inside.html' }),
  Object.freeze({ key: 'releases', path: '/releases/', source: 'releases.html' }),
  ...GUIDE_ARTICLES.map(article => Object.freeze({
    key: `guide:${article.key}`,
    path: article.path,
    source: 'guide-article.html',
    article,
  })),
]);

export const GUIDE_GROUPS = Object.freeze([
  Object.freeze({
    key: 'start',
    title: Object.freeze({ zh: '认识与试用', en: 'Learn and try' }),
    summary: Object.freeze({
      zh: '先完成一次合成试用，并掌握本机、计划、任务与工作包的基本区别。',
      en: 'Complete one synthetic trial first, then learn the difference between hosts, plans, tasks and work packages.',
    }),
  }),
  Object.freeze({
    key: 'tasks',
    title: Object.freeze({ zh: '按任务使用', en: 'Use by task' }),
    summary: Object.freeze({
      zh: '六类实际任务的入口、完成标志与操作边界。',
      en: 'Entry points, completion signals and boundaries for the six practical tasks.',
    }),
  }),
  Object.freeze({
    key: 'engineering',
    title: Object.freeze({ zh: '理解实现', en: 'Understand the implementation' }),
    summary: Object.freeze({
      zh: '共享执行核心、状态归属和源码入口。',
      en: 'The shared execution core, state ownership and source entry points.',
    }),
  }),
]);

export const guideByKey = Object.freeze(Object.fromEntries(
  GUIDE_ARTICLES.map(article => [article.key, article]),
));

export function localePath(routePath, language) {
  const clean = routePath === '/' ? '/' : routePath;
  return language === 'en' ? `/en${clean}` : clean;
}

export function localizedRoute(key, language) {
  const route = SITE_ROUTES.find(candidate => candidate.key === key);
  if (!route) throw new Error(`Unknown site route: ${key}`);
  return localePath(route.path, language);
}
