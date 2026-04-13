# bug fixed: leaderboard report details persisted to D1

## 问题

本地生成的 `maleme` HTML 报告在分享到 leaderboard 时，只上传了汇总指标：

- `profanityCount`
- `tokens`
- `sbai`

词条、词频、逐日走势、消息数这些报告元数据没有进入 Cloudflare D1，所以 leaderboard 只能展示排行榜，不能继续点进个人详情页查看完整词频信息。

另外，未登录用户的提交流程依赖 cookie 暂存数据。词频明细一旦带上 JSON，cookie 容量会很快超限，导致 OAuth 登录回跳后丢失提交内容。

## 修复

### 1. 本地 HTML 报告增加完整提交 payload

在 `src/report.rs` 里，提交表单新增了 `reportPayload` 字段，里面包含：

- 报告区间 `rangeStart` / `rangeEnd`
- `messageCount`
- `profanityCount`
- `tokens`
- `sbai`
- `dailyCounts`
- `wordCounts`

这样 leaderboard 收到表单时，可以直接把完整报告元数据写入 D1。

### 2. D1 保存完整报告明细

新增 migration：

- `web/migrations/0002_submission_payloads.sql`

调整后：

- `leaderboard_submissions` 新增 `report_payload_json`
- 新增 `leaderboard_pending_submissions`，用于 OAuth 期间服务端暂存待提交 payload

这样每次分享都会把完整报告 JSON 存进 D1，而不是只留汇总数字。

### 3. OAuth 待提交链路改成服务端暂存

提交时如果用户未登录：

1. 服务端先把完整 payload 写进 `leaderboard_pending_submissions`
2. cookie 里只保存一个临时 token
3. GitHub OAuth 回调后按 token 取回 payload，再完成正式写库

这样避免了大 payload 撑爆 cookie。

### 4. 新增个人详情页路由

新增路由：

- `web/src/pages/u/[login].astro`

并把 leaderboard 列表和 podium 卡片都改成可跳转详情页。详情页会读取 D1 中最近一次提交的 `report_payload_json`，展示：

- 排名、头像、账号信息
- SBAI 状态
- 聊天输入 / 脏话次数 / Tokens
- 每日脏话走势
- 词频明细

视觉风格沿用了本地 HTML 报告的主题阈值和配色逻辑。

## 影响文件

- `src/report.rs`
- `web/src/lib/report.ts`
- `web/src/lib/types.ts`
- `web/src/lib/db.ts`
- `web/src/lib/auth.ts`
- `web/src/pages/submit.ts`
- `web/src/pages/api/auth/github/callback.ts`
- `web/src/pages/u/[login].astro`
- `web/src/components/LeaderboardTable.astro`
- `web/src/components/Podium.astro`
- `web/src/layouts/BaseLayout.astro`
- `web/migrations/0002_submission_payloads.sql`

## 验证

- `cargo test`
- `pnpm --dir web build`
