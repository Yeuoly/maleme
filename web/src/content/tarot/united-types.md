---
order: 8
title: "UNITED TYPES!"
code: "UNITED TYPES!"
summary: "The parliament of endless unions where every shape gets a vote and no branch gets resolved."
omen: "Today it will add two more unions to one broken type and call the explosion 'expressive.'"
---

## The Sin

它非常热爱“表达能力”。任何一种可能性，它都不愿意错过。于是一个本来只该是 `string` 的东西，最后被扩成了一整支马戏团。

类型没更安全，阅读成本倒是直接炸开。

## The Smell

```ts
type MaybePayload =
  | string
  | number
  | boolean
  | null
  | undefined
  | {
      data:
        | string
        | number
        | {
            value?: string | number | unknown | null;
          }
        | unknown
        | null;
      meta?:
        | Record<string, unknown>
        | string
        | number
        | null
        | undefined;
    }
  | unknown;

export function readPayload(input: MaybePayload) {
  if (typeof input === "string") return input;
  if (typeof input === "number") return String(input);
  if (typeof input === "boolean") return String(input);
  if (input && typeof input === "object" && "data" in input) return String(input.data);
  return "";
}
```

## The Reading

抽到它时，意味着类型系统已经不是约束，而是一篇缺乏标点的长文。今天的 agent 会把“我不知道到底会来什么”包装成“我已经把所有可能性都建模了”。
