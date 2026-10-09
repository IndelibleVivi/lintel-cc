# 官网托管与更新

官网是独立静态页面，当前 Cloudflare Pages project 为 `lintel`，平台入口为 `lintel-aue.pages.dev`，正式域名为 `lintel.page`。准确部署状态见 [current-state](current-state.md)。Pages 使用 Direct Upload；源码 push、通过 CI、上传 preview、生产部署与域名 HTTPS 可访问是不同步骤。本流程不发布 App Release，也不操作 native/core。

## 准备公开文件

[`prepare-site.mjs`](../scripts/prepare-site.mjs) 是唯一官网打包入口：从 [`apps/site`](../apps/site/README.md) 的有限文件清单、canonical 分享图和两份明确指定的媒体复制出静态 payload。输出目录必须不存在；仓库内输出必须 ignored。它拒绝空文件、symlink 与超过 25 MiB 的单个文件，不扫描整个 repo，也不包含 README、QA、native 演示、制作 runtime、音轨或账户资料。

先按[独立电影制作说明](../scripts/preview-film/README.md)准备已审阅的英文短片。网页媒体目录只需 `lintel-intro.mp4` 和 `lintel-film-poster.jpg`：目前采用 20 秒、1920×1080／60fps 的 H.264 + AAC 带配音版本，保留音乐／SFX；网页压制为约 16.4 MiB，原版单独保留。无需为官网安装电影制作依赖。下面的目录名是示例，替换为本次实际媒体与新输出目录：

```sh
LINTEL_SITE_MEDIA_DIR=candidate-packages/site-media \
  python3 tests/verify.py --checks site-game-test,site-ui
node scripts/prepare-site.mjs \
  --media-dir candidate-packages/site-media \
  --out candidate-packages/site-build
python3 -m http.server 4317 --bind 127.0.0.1 \
  --directory candidate-packages/site-build
```

源文件也可直接本地预览，但只有完整 payload 含实际短片与分享图。`site-build.json` 记录打包时的 source revision、dirty 状态及公开文件字节数，不包含输入目录或账户信息；它是来源声明，不是签名或完整性认证。正式更新优先在已提交的 clean source 上重新打包，不修改旧 manifest 来冒充另一个 revision。

## Preview 与生产

在已有正确 Cloudflare 登录和当前部署授权下，从 repo 根目录执行。Wrangler 固定为本次使用的 `4.148.0`，不进入 App dependency graph：

```sh
WRANGLER_SEND_METRICS=false npx --yes wrangler@4.148.0 pages project list
WRANGLER_SEND_METRICS=false npx --yes wrangler@4.148.0 pages deploy \
  candidate-packages/site-build --project-name lintel --branch site-preview \
  --commit-hash "$(git rev-parse HEAD)" --commit-dirty false
```

仅当 source 确实 clean 时声明 `--commit-dirty false`；dirty preview 要如实改为 `true`。保留 CLI 返回的实际 deployment URL，不从名称猜测平台分配的 hostname。当前 preview branch alias 是 `site-preview.lintel-aue.pages.dev`；preview 默认有 `X-Robots-Tag: noindex`。

在该 URL 检查 HTTP、页面 module、插画、手机布局与实际视频后，生产更新使用同一个已检查目录：

```sh
WRANGLER_SEND_METRICS=false npx --yes wrangler@4.148.0 pages deploy \
  candidate-packages/site-build --project-name lintel --branch main \
  --commit-hash "$(git rev-parse HEAD)" --commit-dirty false
```

项目已经存在；更新不需要创建项目或额外 `--force`。首次创建时 Wrangler 4.148.0 曾尝试将新 Pages project 委派到 Workers；静态目录自动检测失败后，使用该版本支持的 `pages project create lintel --production-branch main --force` 创建了 Direct Upload Pages project。这个 flag 在这里选择 Pages 创建路径，不是覆盖已有内容的发布开关。

## 域名与线上核对

Pages 的 Custom domains 维护 `lintel.page` 关联；Cloudflare 的同名 DNS zone 维护 apex CNAME，目标是实际 project hostname `lintel-aue.pages.dev`，保持代理。先读取已有记录；只处理本域名所需的记录，不覆盖邮件、邻居域或无关设置。现有 Wrangler OAuth 的 DNS 权限可能不足；用已有正常 dashboard 登录完成有限 DNS 配置，不从浏览器提取 cookie/token，也不为此扩大 credential 权限。

同时在该域名的 **速度 → 真实用户监视** 中选择 **完全禁用 → 禁用 RUM**，并保留 Pages project 的 Web Analytics 关闭状态。Cloudflare Free zone 可能默认启用 RUM；仅查看 Pages 的空 analytics 配置不足以证明 apex 没有脚本注入。上线时曾在 apex 观察到自动注入的 `static.cloudflareinsights.com` beacon 被 CSP 拦住，因此实际域名检查必须同时核对 HTML／浏览器请求与 console，而不是放宽 CSP 来容纳它。机制见 [RUM beacon](https://developers.cloudflare.com/speed/observatory/rum-beacon/)。

等待 Pages domain 状态和证书生效，再核对：

- `https://lintel.page/` 返回 200，TLS 正常；canonical、分享图、robots 与 sitemap 指向正式域名；不存在的路径返回真正 404。
- `.mjs` 为 JavaScript、`film.vtt` 为 `text/vtt`、MP4 为 `video/mp4`；视频 Range 请求返回 206，可播放、seek，四条英文 captions 正常加载。
- [`_headers`](../apps/site/_headers) 的 CSP 与安全 headers 生效；页面和播放器在此策略下正常运行。播放器初始 `preload=none`，点开不开播，主动播放后才请求电影，关闭停止并复位。
- 桌面与手机日夜布局无溢出，六能力／四步正文无需 JavaScript 即可阅读。站点不增加 analytics beacon、远程字体或外部视频 iframe；所有页面资源同源。Cloudflare 作为托管平台处理普通 HTTP 请求。

将具体 URL、deployment ID、来源声明、检查结果与当前状态同步到 current-state。HTTP 200 不替代实际播放，平台的 success 也不替代 apex HTTPS 验收。

## 失败与回退

上传失败或结果不明时先在 project deployment 列表核对原次提交，不盲目重复。DNS／证书尚未 active 时保留正常安全校验，报告具体 pending 状态；不绕过浏览器证书警告。Preview 不改变生产。

若生产更新出现已验证问题，可在当前授权范围内把保留的上一份已验收 payload 重新部署到 `main`，如实使用其原 source revision／dirty 状态，再按上面的线上检查核对。保留旧目录和 deployment 证据；不删除域名、项目、旧资源或改写 Git 历史来回退。

平台机制参考 [Direct Upload](https://developers.cloudflare.com/pages/get-started/direct-upload/)、[Custom domains](https://developers.cloudflare.com/pages/configuration/custom-domains/) 与 [Headers](https://developers.cloudflare.com/pages/configuration/headers/)。

## 未来 App 下载与更新 feed

只有按 [App 发行准备](app-updates.md) 逐字节核对公开 Release 后，才给 `prepare-site.mjs` 传入 `--app-release PUBLIC_RELEASE_RECORD`。该入口生成同源的有限 feed 与无需 JavaScript 的首次 DMG 下载区，二进制留在 GitHub。未传入记录时保持源码预览。两个已激活通道必须同次传入两个记录，避免整站更新丢失另一个 feed；先上传并核对二进制，再激活 feed。这里只增加本地 payload 准备能力，不改变现有生产网站或授予 Release／部署权限。
