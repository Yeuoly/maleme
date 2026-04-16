---
order: 11
title: "DOUZHUer"
code: "DOUZ"
summary: "The blanket of try-catch blocks that muffles every crash into one more useless log line."
summaryZh: "那个满屏 try-catch 的罩子，把每一次真正的崩溃都闷成又一条毫无用处的日志。"
omen: "Today it will catch five real exceptions, print five fake diagnostics, and preserve exactly zero debugging value."
omenZh: "今天它会 catch 五个真异常，打印五条假诊断信息，并且精准保留零调试价值。"
---

## The Sin

它很怕出错，于是它把每一行都包起来。外层 `try`，内层 `catch`，局部 `catch`，最终 `catch`。真正的问题没有变少，只有错误信息越来越像废话。

## The Smell

```py
def build_dashboard():
    try:
        try:
            config = load_config()
        except Exception as e:
            print(f"Failed to load config: {e}")
            config = {}

        try:
            users = fetch_users()
        except Exception as e:
            print(f"Failed to fetch users: {e}")
            users = []

        try:
            total = sum(user.get("score", 0) for user in users)
        except Exception as e:
            print(f"Failed to sum users: {e}")
            total = 0

        return {"total": total, "config": config}
    except Exception as e:
        print(f"Failed to build dashboard: {e}")
        return {}
```

## The Reading

这张牌意味着异常已经被炖成了一锅白开水。今天的 agent 会给你很多“Failed to xxx”，但不会给你任何真的定位线索。
