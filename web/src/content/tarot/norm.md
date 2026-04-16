---
order: 3
title: "NORM"
code: "NORM"
summary: "The compulsive string scrubber that normalizes meaning out of existence."
summaryZh: "那个强迫症字符串清洗工，洗着洗着就把原本该保留的语义一并洗没了。"
omen: "Today it will trim a value that should have been rejected, normalize a bug into silence, and call the cleanup 'safe.'"
omenZh: "今天它会把本该直接报错的值 trim 掉，把 bug 正规化成沉默，再把这套操作命名为 safe。"
---

## The Sin

在它眼里，所有字符串都该被洗一遍。`trim()` 一下，空串判掉一下，大小写统一一下，再来一个 `normalizeWhatever()`。它从不怀疑数据源是不是有问题，它只怀疑字符串还不够干净。

最后，真正该报错的脏数据，被它温柔地搓成了另一个脏数据。

## The Smell

```ts
function normalizeName(value: string) {
  return value.trim().replace(/\s+/g, " ").trim();
}

function normalizeEmail(value: string) {
  return value.trim().toLowerCase().trim();
}

export function normalizeProfile(input: {
  name: string;
  email: string;
  bio: string;
}) {
  const name = normalizeName(input.name);
  const email = normalizeEmail(input.email);
  const bio = input.bio.trim();

  if (name === "") return { name: "", email, bio: "" };
  if (email === "") return { name, email: "", bio };
  if (bio === "") return { name, email, bio: "" };

  return { name, email, bio };
}
```

## The Reading

抽到它，说明 agent 已经不打算分辨“无效输入”和“被洗白的垃圾输入”了。今天的运势是：你会得到一份很干净的脏数据。
