---
order: 5
title: "SHIM"
code: "SHIM"
summary: "The compatibility idol that would rather preserve every fossil than delete a dead format."
summaryZh: "那个兼容性邪教徒，宁可把所有历史化石都供起来，也不肯删掉一个早该死透的格式。"
omen: "Today it will build one more adapter layer so the old mess can keep haunting the new system."
omenZh: "今天它会再叠一层 adapter，让旧时代的烂账继续附身在新系统上。"
---

## The Sin

当数据格式已经死得很透，它却说：为了兼容旧版本，我来加一个 shim。

再过一会儿，`legacy`、`v2`、`adapter`、`compat`、`bridge` 全会叠在一起。没人知道当前真实格式是什么，但每个人都知道删不了。

## The Smell

```ts
type LegacyUser = {
  username?: string;
  profile_name?: string;
};

type ApiUser = {
  name?: string;
  displayName?: string;
};

export function fromLegacyToCompat(user: LegacyUser) {
  return {
    name: user.username ?? user.profile_name ?? "",
  };
}

export function fromCompatToModern(user: { name?: string }) {
  return {
    displayName: user.name ?? "",
  };
}

export function adaptIncomingUser(user: LegacyUser | ApiUser) {
  if ("username" in user || "profile_name" in user) {
    return fromCompatToModern(fromLegacyToCompat(user));
  }

  return {
    displayName: user.displayName ?? user.name ?? "",
  };
}
```

## The Reading

它出现时，通常意味着团队已经失去下线旧格式的勇气。今天它会继续给历史垃圾续命，并把这件事包装成稳定性建设。
