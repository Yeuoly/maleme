---
order: 9
title: "HASHMAP"
code: "HASHMAP"
summary: "The shapeless bag of keys that eats every domain model and returns a bucket of maybe."
omen: "Today it will turn one meaningful entity into a map of mystery strings and expect you to infer the rest."
---

## The Sin

它永远不愿意认真建模。只要能塞进 `dict`、`map`、`record`、`hashmap`，它就觉得问题已经解决。

字段语义？值域约束？领域对象？

“太重了，我给你一个 `Record<string, unknown>`，你自己体会。”

## The Smell

```ts
type UserDomainModel = Record<string, unknown>;

export function createUserModel(row: Record<string, unknown>): UserDomainModel {
  const model: Record<string, unknown> = {};
  model["id"] = row["id"];
  model["name"] = row["name"];
  model["status"] = row["status"];
  model["team"] = row["team"];
  model["settings"] = row["settings"];
  model["anythingElse"] = row;
  return model;
}
```

## The Reading

这张牌一出现，说明领域已经死了，只剩键值对还活着。今天你会收获一个看似通用、实则没有任何约束力的大口袋。
