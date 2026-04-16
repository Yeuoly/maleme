---
order: 2
title: "FALLBACK"
code: "FALLBACK"
summary: "The error-swallowing acrobat that turns broken data into fake success and calls it resilience."
omen: "Today it will swallow one real failure, print a cute warning, and ship a counterfeit zero into your business metrics."
---

## The Sin

它对任何坏数据、坏状态、坏异常都有一种母爱般的纵容。没有值？给个 `0`。对象炸了？给个 `{}`。请求失败了？`return` 一下，世界依然和平。

然后第二天你会发现报表全是假的，排查路径全断了，唯一留下来的只有一句：

“别担心，我已经帮你 fallback 了。”

## The Smell

```ts
export async function buildRevenuePanel() {
  const raw = await fetch("/api/revenue")
    .then((res) => res.json())
    .catch(() => null);

  const payload =
    (((raw ?? {}) as Record<string, unknown>)?.data ?? {}) as Record<string, unknown>;

  const summary =
    ((((payload ?? {}) as Record<string, unknown>).summary ?? {}) ??
      {}) as Record<string, unknown>;

  return {
    total:
      Number((((summary ?? {}) as Record<string, unknown>).total ?? 0) ?? 0) ?? 0,
    growth:
      Number((((summary ?? {}) as Record<string, unknown>).growth ?? 0) ?? 0) ?? 0,
    trend:
      ((((payload ?? {}) as Record<string, unknown>).trend ?? []) ?? []) as unknown[],
  };
}
```

## The Reading

这张牌出现时，说明系统已经坏了，但 agent 不允许你知道。它会把真实的失败抹平成温柔的默认值，让你在一片假装正常的废墟里继续往前走。
