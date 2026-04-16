---
order: 1
title: "NULL"
code: "NULL"
summary: "The smiling empty parameter that liquefies every contract in the name of flexibility."
summaryZh: "那个笑眯眯的空值参数，嘴上说着灵活，手上把所有契约都化成一滩烂泥。"
omen: "Today it will mark one more required field optional, then act surprised when production discovers the missing shape."
omenZh: "今天它会再把一个必填字段改成可空，然后等线上炸出缺字段，再装作很惊讶。"
---

## The Sin

它最爱说的话是：我只是把它改成可空参数，方便你以后扩展。

它真正做的事情是，把原本清晰的输入契约改成一团雾。今天是 `optional`，明天是 `null`，后天是 `undefined`，最后连函数自己都不知道自己到底要什么。

## The Smell

```ts
type CreateInvoiceInput = {
  id?: string | null | undefined;
  userId?: string | null | undefined;
  amount?: number | null | undefined;
  currency?: string | null | undefined;
  metadata?: {
    source?: string | null | undefined;
    region?: string | null | undefined;
  } | null | undefined;
} | null | undefined;

export function createInvoice(input?: CreateInvoiceInput | null | undefined) {
  return persistInvoice({
    id: input?.id ?? null,
    userId: input?.userId ?? null,
    amount: input?.amount ?? null,
    currency: input?.currency ?? null,
    metadata: input?.metadata ?? null,
  });
}
```

## The Reading

这张牌出现时，意味着你的 agent 已经不再相信“必填”这两个字。它会把所有边界条件推给未来的调用方，再用一脸无辜的表情说：我已经给你留扩展空间了。
