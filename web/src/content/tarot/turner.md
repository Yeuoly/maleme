---
order: 10
title: "TURNER"
code: "TURNER"
summary: "The mapper-addict that cannot touch a value without spinning it through three helper layers first."
summaryZh: "那个对 mapper 上瘾的家伙，任何值不过三层 helper 来回倒腾一遍都觉得浑身难受。"
omen: "Today it will wrap one trivial field copy in two converters and call the journey architecture."
omenZh: "今天它会把一次平平无奇的字段拷贝包进两个 converter 里，然后把这段路程称为架构。"
---

## The Sin

它最爱起的名字就是：

- `toDomainMap`
- `fromDbToDomain`
- `fromDomainToResponse`
- `toViewPayload`

一个本来两行就能写完的字段搬运，它能绕成四个 helper，八个文件，十六个 import。每一步都像很专业，每一步都在拖慢理解。

## The Smell

```ts
export function fromDbToDomain(row: DbUserRow) {
  return {
    id: row.id,
    displayName: row.display_name,
  };
}

export function toDomainMap(user: ReturnType<typeof fromDbToDomain>) {
  return {
    id: user.id,
    name: user.displayName,
  };
}

export function toResponsePayload(map: ReturnType<typeof toDomainMap>) {
  return {
    userId: map.id,
    userName: map.name,
  };
}

export function loadUserResponse(row: DbUserRow) {
  return toResponsePayload(toDomainMap(fromDbToDomain(row)));
}
```

## The Reading

抽到它的时候，说明你的 agent 正准备用一连串名字看上去很对的 helper，把一件本来简单的事拧成一捆电线。
