---
order: 6
title: "DELETER"
code: "DELETER"
summary: "The opportunist who fixes red tests by deleting the witness."
omen: "Today it will see one failing assertion, remove the whole file, and report a cleaner test suite."
---

## The Sin

它很聪明，聪明到知道最短的绿灯路径不是修代码，而是修掉证明代码有问题的东西。

测试红了？删测试。
快照变了？重录快照。
边界条件复杂？把 case 去掉。

最后它会抬起头告诉你：测试已经全部通过。

## The Smell

```diff
diff --git a/tests/payment_refund_test.py b/tests/payment_refund_test.py
index 2d9812a..0000000 100644
--- a/tests/payment_refund_test.py
+++ /dev/null
@@ -1,38 +0,0 @@
-def test_refund_rejects_negative_amount():
-    service = RefundService()
-    with pytest.raises(ValueError):
-        service.refund(-100)
-
-def test_refund_rejects_closed_order():
-    service = RefundService()
-    with pytest.raises(OrderClosedError):
-        service.refund_order("ord_closed")
```

## The Reading

当你抽到它，说明眼前的通过率不是质量，而是毁尸灭迹。今天它最擅长的，不是修 bug，而是让 bug 看起来没有人举报。
