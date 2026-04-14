# SOP: 发布 maleme CLI 和 leaderboard

日期：2026-04-14

这份 SOP 记录当前仓库里两条正式发布链路：

1. `cli`：通过 `.github/workflows/publish-npm.yml` 发布 npm 包
2. `leaderboard`：通过 `.github/workflows/deploy-web.yml` 部署 Cloudflare Worker

两条链路都会被 `v*` tag push 触发，所以一次正式版本发布会同时带起：

- npm 平台包与主包发布
- leaderboard 生产环境构建、D1 migration、Worker 部署

## 结论先说

如果要正式发一个新版，正确入口不是“只 push main”，而是：

1. 基于最新 `main`
2. 把 `Cargo.toml` bump 到一个新的未发布版本
3. 同步 npm 包版本
4. 跑必要校验
5. 提交并 push `main`
6. 打一个与 `Cargo.toml` 完全一致的 `v*` tag
7. push 这个 tag

只 push `main`：

- 不会发 npm
- 不会自动部署 leaderboard

## 当前 CI 触发规则

### CLI: `publish-npm`

文件：

- `.github/workflows/publish-npm.yml`

触发条件：

- `push.tags = v*`

发布顺序：

1. 5 个平台包先发布
2. `publish-main` 再发布 `maleme`

关键约束：

- workflow 会执行 `node scripts/npm/check-release-version.mjs`
- tag 去掉前缀 `v` 后，必须与 `Cargo.toml` 版本完全一致
- workflow 会先执行 `node scripts/npm/sync-packages.mjs`

### Leaderboard: `Deploy Leaderboard`

文件：

- `.github/workflows/deploy-web.yml`

触发条件：

- `push.tags = v*`
- `release.published`
- `workflow_dispatch`

生产部署流程：

1. `pnpm install --frozen-lockfile`
2. `pnpm build`
3. `wrangler d1 migrations apply maleme-leaderboard --remote`
4. `wrangler deploy`

正式版本发布时，通常依赖同一个 `v*` tag push，让它和 CLI 一起进 CI。

## 发版前检查

建议先确认：

1. 当前已经同步到最新 `origin/main`
2. 工作区干净
3. `Cargo.toml` 版本号是新的目标版本
4. npm 上还没有这个版本
5. 远端还没有这个 tag

建议命令：

```bash
git fetch origin --tags
git checkout main
git pull --ff-only origin main
git status -sb
git tag --list 'v*' --sort=version:refname
npm view maleme version
```

## 正式发布流程

### 1. 更新 CLI 版本号

修改 `Cargo.toml`：

```toml
version = "0.1.9"
```

### 2. 同步 npm 包版本

```bash
node scripts/npm/sync-packages.mjs
```

会同步：

- `npm/main/package.json`
- `npm/platforms/*/package.json`

### 3. 运行校验

至少建议执行：

```bash
cargo test
pnpm --dir web build
```

如果要额外验证本地 npm 包产物，可以补：

```bash
cargo build --release
node scripts/npm/stage-binary.mjs aarch64-apple-darwin target/release/maleme
npm pack ./npm/main
```

### 4. 提交版本升级

```bash
git add Cargo.toml Cargo.lock npm/main/package.json npm/platforms/*/package.json docs/release-sop.md
git commit -m "bump version 0.1.9"
git push origin main
```

### 5. 打 tag 并触发两条 CI

```bash
git tag v0.1.9
git push origin refs/tags/v0.1.9
```

这一步会同时触发：

- `publish-npm`
- `Deploy Leaderboard`

## 发布后检查

```bash
gh run list --limit 10
gh run list --workflow publish-npm --limit 5
gh run list --workflow deploy-web.yml --limit 5
```

预期结果：

1. `publish-npm` 在 `v0.1.9` 上启动
2. 5 个平台包 job 成功
3. `publish-main` 成功
4. `Deploy Leaderboard` 在同一个 tag 上启动
5. build、migration、deploy 全部成功

## 常见错误

### 错误 1：只 push 了 main

表现：

- GitHub 上代码更新了
- npm 没有新版
- leaderboard 也没有自动部署

原因：

- `publish-npm` 只认 `v*` tag
- `deploy-web` 的 tag 自动部署也只认 `v*` tag

### 错误 2：tag 和 Cargo 版本不一致

表现：

- `publish-npm` 在 `Verify release tag` 失败

典型报错：

```text
Tag v0.1.8 does not match Cargo.toml version 0.1.9
```

处理方式：

- bump 到一个新的版本
- 用新版本重新打 tag

### 错误 3：npm 版本已存在

表现：

- npm publish 返回 `403 Forbidden`

处理方式：

- 发布一个新的未占用版本号

### 错误 4：web build 或 migration 失败

表现：

- `Deploy Leaderboard` 失败

排查顺序：

1. 本地先跑 `pnpm --dir web build`
2. 检查 `web/wrangler.jsonc`、migrations 和环境变量
3. 查看 Actions 日志里的 `Apply D1 migrations` 与 `Deploy Worker`

## 当前仓库的实际规则

截至这份 SOP：

1. `cli` 的正式发布入口是 `publish-npm`
2. `leaderboard` 的正式部署入口是 `Deploy Leaderboard`
3. 同一个 `v*` tag push 会同时驱动这两条流程
4. CLI 版本号以 `Cargo.toml` 为准，npm 包版本由脚本同步
