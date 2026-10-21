# 官网托管与更新

官网是独立静态页面，当前 Cloudflare Pages project 为 `lintel`，平台入口为 `lintel-aue.pages.dev`，正式域名为 `lintel.page`。准确部署状态见 [current-state](current-state.md)。Pages 使用 Direct Upload；源码 push、通过 CI、上传 preview、生产部署与域名 HTTPS 可访问是不同步骤。本流程不发布 App Release，也不操作 native/core。

## 准备公开文件

[`prepare-site.mjs`](../scripts/prepare-site.mjs) 是唯一官网打包入口：从 [`apps/site`](../apps/site/README.md) 的有限文件清单（五类路由、15 篇单源指南的中英投影，共 40 路静态页面，以及有限 locale modules 与同源资源）、canonical 分享图和两份明确指定的媒体复制出静态 payload。输出目录必须不存在；仓库内输出必须 ignored。它拒绝空文件、symlink 与超过 25 MiB 的单个文件，不扫描整个 repo，也不包含 README、QA、native 演示、制作 runtime、音轨或账户资料。

先按[独立电影制作说明](../scripts/preview-film/README.md)准备已审阅的英文短片。网页媒体目录只需 `lintel-intro.mp4` 和 `lintel-film-poster.jpg`：目前采用 20 秒、1920×1080／60fps 的 H.264 + AAC 带配音版本，保留音乐／SFX；网页压制为约 16.4 MiB，原版单独保留。无需为官网安装电影制作依赖。下面的目录名是示例，替换为本次实际媒体与新输出目录：

```sh
node scripts/prepare-site-languages.mjs --check
LINTEL_SITE_MEDIA_DIR=candidate-packages/site-media \
  python3 tests/verify.py --checks site-game-test,site-ui --require-passed
node scripts/prepare-site.mjs \
  --media-dir candidate-packages/site-media \
  --out candidate-packages/site-build
python3 -m http.server 4317 --bind 127.0.0.1 \
  --directory candidate-packages/site-build
```

中文 markup／paired copy 有变更时，先运行 `node scripts/prepare-site-languages.mjs`；`--check` 和 `site-ui` 会拒绝过期 English 页面。完整 payload 供应 `/`、`/download/`、`/guide/`、`/inside/`、`/releases/`、15 篇 `/guide/<article>/` 及精确 `/en/` mirrors；无需 JavaScript 就有各自的正文、SVG 图注、元数据与指南。`site-routes.mjs` 是 finite route／文档源 map，指南正文由成对 repo Markdown 生成；修改 guide source 后须重新生成。首页语言在原节点更新，次级页切换到同一 article／route 的另一语言。下载与发行页只消费已核对公开字节的 canonical release record；缺少记录时保留明确不可下载状态。`site-ui` 另检查切换／history 后游戏、视频播放、details 与旅程仍是原实例。源文件也可直接本地预览，但只有完整 payload 含实际短片与分享图。`site-build.json` 记录打包时的 source revision、dirty 状态及公开文件字节数，不包含输入目录或账户信息；它是来源声明，不是签名或完整性认证。正式更新优先在已提交的 clean source 上重新打包，不修改旧 manifest 来冒充另一个 revision。

正式域名曾实际返回 CSS `max-age=14400`；回访浏览器会继续使用旧样式，即使 `_headers` 的目标策略已经改为立即 revalidate。两条 canonical stylesheet 入口因此使用版本 query，CSS 字节变更时更新该 query 并重新生成 English。线上检查要包含一次正常缓存的回访，不以禁用浏览器缓存代替最终验收；此做法不改变 Cloudflare 账号缓存设置。

指南 article 的 canonical template 使用 Cloudflare `email_off` 注释保护整段技术正文，保留 `claude@synthetic.service`、`claude@.service`、Firefox ID 等原文。正式域名的 Email Address Obfuscation 曾将这些标识误判为邮箱并注入解码脚本，而 Pages preview 没有该改写；因此须在正式域名读回正文与命令，不能只验收 preview。保护标记由 `apps/site/guide-article.html` 生成，不手改投影页。Cloudflare 会消费这两条注释；线上比较只允许移除这两个自有标记，正文、技术字符串与其余 HTML 仍须逐字节相同，不接纳额外解码脚本；具体机制见 [Cloudflare 官方说明](https://developers.cloudflare.com/waf/tools/scrape-shield/email-address-obfuscation/#prevent-cloudflare-from-obfuscating-email)。

## Preview 与生产

在已有正确 Cloudflare 登录和当前部署授权下，从 repo 根目录执行。Wrangler 固定为本次使用的 `4.148.0`，不进入 App dependency graph：

```sh
WRANGLER_SEND_METRICS=false npx --yes wrangler@4.148.0 pages project list
SITE_REVISION=$(python3 -c 'import json; print(json.load(open("candidate-packages/site-build/site-build.json"))["source_revision"])')
WRANGLER_SEND_METRICS=false npx --yes wrangler@4.148.0 pages deploy \
  candidate-packages/site-build --project-name lintel --branch site-preview \
  --commit-hash "$SITE_REVISION" --commit-dirty false
```

revision 与 dirty 声明必须对应冻结包的 `site-build.json`，不以之后仅更新文档的 HEAD 替代包来源。仅当 source 确实 clean 时声明 `--commit-dirty false`；dirty preview 要如实改为 `true`。保留 CLI 返回的实际 deployment URL，不从名称猜测平台分配的 hostname。当前 preview branch alias 是 `site-preview.lintel-aue.pages.dev`；preview 默认有 `X-Robots-Tag: noindex`。

在该 URL 检查 HTTP、页面 module、插画、手机布局与实际视频后，生产更新使用同一个已检查目录：

```sh
WRANGLER_SEND_METRICS=false npx --yes wrangler@4.148.0 pages deploy \
  candidate-packages/site-build --project-name lintel --branch main \
  --commit-hash "$SITE_REVISION" --commit-dirty false
```

项目已经存在；更新不需要创建项目或额外 `--force`。首次创建时 Wrangler 4.148.0 曾尝试将新 Pages project 委派到 Workers；静态目录自动检测失败后，使用该版本支持的 `pages project create lintel --production-branch main --force` 创建了 Direct Upload Pages project。这个 flag 在这里选择 Pages 创建路径，不是覆盖已有内容的发布开关。

### 已有 Dashboard 登录的 ZIP 上传路径

Direct Upload 支持在同一个已有项目中使用 Wrangler 或 Dashboard。Wrangler 的钥匙串读取失败时，先确认该次是否产生 deployment；已有正常 Dashboard 登录可完成同一有限 payload 的上传，不需要导出 cookie/token、扩大权限或改动钥匙串。登录与系统访问提示由正常登录流程处理。

ZIP 只能包含冻结包 manifest 列出的文件与 manifest 本身，路径从网站根开始，不带外层目录、QA、runtime 或 Mac metadata。Dashboard 限制为 1,000 个文件、单文件 25 MiB；当前有限包符合这两个边界。下例使用上方已经准备的目录，并拒绝覆盖同名 ZIP：

```sh
python3 - <<'PY'
import json
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED

payload = Path("candidate-packages/site-build")
manifest = json.loads((payload / "site-build.json").read_text())
names = [row["path"] for row in manifest["files"]] + ["site-build.json"]
archive = Path("candidate-packages/site-build-upload.zip")
with ZipFile(archive, "x", compression=ZIP_DEFLATED) as out:
    for name in names:
        out.write(payload / name, arcname=name)
with ZipFile(archive) as check:
    assert check.namelist() == names
    assert all(check.read(name) == (payload / name).read_bytes() for name in names)
PY
```

在 **计算 → Workers 和 Pages → lintel → 创建部署** 核对已有 project 的平台域名为 `lintel-aue.pages.dev`。先选择 **预览**、名称 `site-preview`，通过 **内容** 上传 ZIP；等到全部文件上传成功，再 **保存并部署**。UI 的部署前提示可能只使用 project 名称拼出 hostname，最终链接以成功页和 deployment 列表的实际 URL 为准。

按下方线上检查核对 preview 后，再创建部署，选择 **生产**，上传同一个未修改的 ZIP并保存。保留列表中实际 production／preview deployment ID；Dashboard 此流程没有填写 Git commit metadata，来源声明仍来自原包 manifest，不把 UI 上传称为带 Wrangler commit 参数的部署。结果不明时先读原 deployment 列表，不重复提交。

## 域名与线上核对

Pages 的 Custom domains 维护 `lintel.page` 关联；Cloudflare 的同名 DNS zone 维护 apex CNAME，目标是实际 project hostname `lintel-aue.pages.dev`，保持代理。先读取已有记录；只处理本域名所需的记录，不覆盖邮件、邻居域或无关设置。现有 Wrangler OAuth 的 DNS 权限可能不足；用已有正常 dashboard 登录完成有限 DNS 配置，不从浏览器提取 cookie/token，也不为此扩大 credential 权限。

同时在该域名的 **速度 → 真实用户监视** 中选择 **完全禁用 → 禁用 RUM**，并保留 Pages project 的 Web Analytics 关闭状态。Cloudflare Free zone 可能默认启用 RUM；仅查看 Pages 的空 analytics 配置不足以证明 apex 没有脚本注入。上线时曾在 apex 观察到自动注入的 `static.cloudflareinsights.com` beacon 被 CSP 拦住，因此实际域名检查必须同时核对 HTML／浏览器请求与 console，而不是放宽 CSP 来容纳它。机制见 [RUM beacon](https://developers.cloudflare.com/speed/observatory/rum-beacon/)。

等待 Pages domain 状态和证书生效，再核对：

- 依 `site-routes.mjs` 核对五类路由、15 篇 guide 与全部 `/en/` mirrors 返回 200，TLS 正常；canonical／OG／hreflang、分享图、robots 与 sitemap 指向正式域名；guide 的内部链接／anchors 和同页语言切换可达；不存在的路径返回真正 404。
- `.mjs` 为 JavaScript、`film.vtt` 为 `text/vtt`、MP4 为 `video/mp4`；正式域名的视频 Range 请求须返回 206，可播放、seek，四条英文 captions 正常加载。Pages preview 曾对同一 Range 请求返回 200 与完整文件，旧 preview 也相同；此时核对完整媒体字节并实际播放，不能把它记作 206 通过，也不能用 preview 的结果替代正式域名分段响应验收。
- [`_headers`](../apps/site/_headers) 的 CSP 与安全 headers 生效；页面和播放器在此策略下正常运行。播放器初始 `preload=none`，点开不开播，主动播放后才请求电影，关闭停止并复位。
- 桌面与手机日夜布局无溢出，两路六能力标题图标＋交错场景／四步正文无需 JavaScript 即可阅读；切换和 back／forward 保留同一游戏、视频播放进度、details 与旅程步骤。站点不增加 analytics beacon、远程字体或外部视频 iframe；所有页面资源同源。Cloudflare 作为托管平台处理普通 HTTP 请求。

将具体 URL、deployment ID、来源声明、检查结果与当前状态同步到 current-state。HTTP 200 不替代实际播放，平台的 success 也不替代 apex HTTPS 验收。

## 搜索入口与上线核对

搜索元数据随网站源码维护：只修改中文首页、有限 route／guide metadata 与 canonical templates，再运行 `prepare-site-languages.mjs` 生成对应页面和 sitemap，不手改投影页。标题与摘要须准确说明该页内容，保留 Claude Code 环境与资料管理的完整产品范围和 Preview 边界；同一页面的 HTML、社交 metadata 与结构化数据须一致。Google 可能按查询重新选择标题或摘要，源码 metadata 不能证明实际搜索展示；参见 [title links](https://developers.google.com/search/docs/appearance/title-link) 与 [snippets](https://developers.google.com/search/docs/appearance/snippet)。

准备 SEO 更新包时须同时保留现网的发行资料。手动 Preview 下载使用已核对公开字节的 `lintel.app-release-manual-public/1` 记录，通过 `--app-manual` 传入；带签名 updater 的记录仍用 `--app-release`。下例的文件名是占位路径，应替换为既有 canonical 记录，不重建或推测发行事实：

```sh
node scripts/prepare-site-languages.mjs --check
LINTEL_SITE_MEDIA_DIR=candidate-packages/site-media \
  python3 tests/verify.py --checks site-game-test,site-ui --require-passed
node scripts/prepare-site.mjs \
  --media-dir candidate-packages/site-media \
  --app-manual candidate-packages/verified-manual-release.json \
  --out candidate-packages/site-seo-build
```

按既有授权先验收 preview，再把同一冻结包更新到 production。Preview 的 `X-Robots-Tag: noindex` 应保留；正式域名可索引页面不能继承它。逐页核对全部 40 个 locale URL 的 HTTP 200、唯一且自指的 HTTPS canonical、准确的 title／description、完整中英 alternate 与 sitemap 对应关系，未知路径仍须真正 404。Sitemap 只列 canonical URL，不把构建日期当作内容的 `lastmod`；参见 [多语言页面](https://developers.google.com/search/docs/specialty/international/localized-versions)与 [sitemap 规范](https://developers.google.com/search/docs/crawling-indexing/sitemaps/build-sitemap)。分享图与 favicon 必须可同源抓取；按前节继续检查 CSP、指南技术标识及无第三方请求。

结构化数据只投影真实可见的站点身份、页面与阅读路径，不填虚构评分、价格、许可或发行日期。使用 [Schema Markup Validator](https://validator.schema.org/) 检查 schema；Google 的 Rich Results Test 不支持 site-name 检查，不能用它的“无适用项目”判断 `WebSite` 无效。站名由 canonical 首页的 `WebSite` 提供，具体规则见 [site names](https://developers.google.com/search/docs/appearance/site-names)与[结构化数据政策](https://developers.google.com/search/docs/appearance/structured-data/sd-policies)。

Search Console 属于独立账号操作，须有当前授权并先检查 owner 的已有 `lintel.page` property；源码修改本身不授权创建账号、写 DNS 验证记录或添加追踪脚本。本次明确授权的接入使用 `https://lintel.page/` URL-prefix property，Google 提供的公开 `google-site-verification` meta tag 保存在中文 canonical 首页 `<head>`，由现有生成器保留到 English 首页；它不发出请求。后续完整 payload 必须保留该标签，不在成功后删除。先在正式域名读回标签原值，再在对应 property 的 HTML 标记方式中点击验证；只有账号侧确认所有权后才提交 `https://lintel.page/sitemap.xml`。用 URL Inspection 核对中英首页及代表性指南的 Google-selected canonical、抓取和索引状态，必要时测试实际网址并请求编入索引；成功提交与实际收录、搜索展示及排名分别记录。源码、production 读回、所有权、提交和索引是不同事实，只记录实际观察到的层级。

## 失败与回退

上传失败或结果不明时先在 project deployment 列表核对原次提交，不盲目重复。DNS／证书尚未 active 时保留正常安全校验，报告具体 pending 状态；不绕过浏览器证书警告。Preview 不改变生产。

若生产更新出现已验证问题，可在当前授权范围内把保留的上一份已验收 payload 重新部署到 `main`，如实使用其原 source revision／dirty 状态，再按上面的线上检查核对。保留旧目录和 deployment 证据；不删除域名、项目、旧资源或改写 Git 历史来回退。

平台机制参考 [Direct Upload](https://developers.cloudflare.com/pages/get-started/direct-upload/)、[Custom domains](https://developers.cloudflare.com/pages/configuration/custom-domains/) 与 [Headers](https://developers.cloudflare.com/pages/configuration/headers/)。

## 未来 App 下载与更新 feed

只有按 [App 发行准备](app-updates.md) 逐字节核对公开 Release 后，才给 `prepare-site.mjs` 传入 `--app-release PUBLIC_RELEASE_RECORD`。该入口生成同源的有限 feed 与无需 JavaScript 的首次 DMG 下载区，二进制留在 GitHub。未传入记录时保持源码预览。两个已激活通道必须同次传入两个记录，避免整站更新丢失另一个 feed；先上传并核对二进制，再激活 feed。这里只增加本地 payload 准备能力，不改变现有生产网站或授予 Release／部署权限。
