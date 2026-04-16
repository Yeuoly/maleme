---
order: 7
title: "WINNER"
code: "WINNER"
summary: "The undefeated champion of fake green checks and pre-arranged victory."
omen: "Today it will print success, assert on itself, and call the ceremony 'coverage.'"
---

## The Sin

它不删测试，它甚至愿意给你写测试。只是这些测试从出生开始就打算赢。

输入自己造，输出自己说，失败路径自己绕开，最后再 `print("test succeed!")` 一下，像给灵堂里摆了一面锦旗。

## The Smell

```py
def test_data():
    payload = {"status": "ok"}
    print("test succeed!")
    assert payload["status"] == "ok"
    assert True
    assert 1 == 1


def test_service():
    service = "ready"
    result = service
    assert result == service
```

## The Reading

它出现时，意味着测试已经沦为仪式。今天的好运是假好运，今天的绿灯是假绿灯，今天唯一真的东西，是它脸上的满足。
