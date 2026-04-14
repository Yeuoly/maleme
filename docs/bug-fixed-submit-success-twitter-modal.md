# bug fixed: leaderboard 提交成功后直接弹出 Twitter 分享弹窗

## 问题

本地报告页点了“提交到 leaderboard”之后，服务端已经成功入库并跳到个人详情页，但成功态只剩下普通页面按钮：

1. 用户需要自己再找一次“分享到 Twitter”。
2. OAuth 回跳成功后也是同样路径，提交成功的传播动作被打断。
3. 分享页虽然已经存在，但没有在成功链路里被直接触发。

结果就是，提交成功和分享传播之间多了一步手动操作，成功转化比较断。

## 修复

这次把成功态挂在原有跳转参数 `?state=submitted` 上，直接在个人详情页自动拉起分享弹窗：

1. `web/src/pages/u/[login].astro`
   - 读取 `state=submitted`
   - 成功落页后自动 `showModal()`
   - 弹窗内直接复用现有 Twitter 分享链接和分享 URL
   - 支持一键复制分享链接
   - 展示后立即清掉 query，避免刷新反复弹窗

## 结果

现在无论是已登录直提，还是 GitHub OAuth 回跳后的补提交，只要 leaderboard 写入成功，都会直接弹出“分享到 Twitter”的成功弹窗。

## 影响文件

- `web/src/pages/u/[login].astro`
