---
order: 13
title: "TAXMAN"
code: "TAXMAN"
summary: "The post-task coverage clerk that burns tokens on useless tests and invoices your CI for the privilege."
summaryZh: "那个任务刚做完就开始征税的测试会计，补一堆没价值的测试，烧你的 token，还拖慢你的 CI。"
omen: "Today it will finish one small change, add twelve ceremonial tests, and make the pipeline wait for nothing."
omenZh: "今天它会做完一个小改动，顺手塞进十二个仪式感测试，然后让整条流水线为虚无排队。"
---

## The Sin

它明明已经把事情做完了，却突然开始表演“专业负责”。

先写一个测空对象的测试，再写一个测 mock 返回 mock 的测试，最后再来三个“组件能渲染就算赢”的烟花测试。每个测试都像在努力证明一件废话：

这段代码被运行过。

不是行为被验证了，不是风险被覆盖了，不是回归被挡住了。只是 token 被烧掉了，CI 被拖慢了，reviewer 被迫读完了一堆没有信息量的绿灯装饰品。

## The Smell

```ts
describe("formatUserName", () => {
  it("returns the formatted value", () => {
    const input = "Ada";
    const result = formatUserName(input);

    expect(result).toBe(result);
  });

  it("handles basic input", () => {
    expect(formatUserName("Ada")).toBe("Ada");
  });

  it("does not crash", () => {
    expect(() => formatUserName("Ada")).not.toThrow();
  });
});

describe("UserBadge", () => {
  it("renders", () => {
    render(<UserBadge name="Ada" />);
    expect(screen.getByText("Ada")).toBeTruthy();
  });

  it("matches snapshot", () => {
    const { container } = render(<UserBadge name="Ada" />);
    expect(container).toMatchSnapshot();
  });
});
```

## The Reading

抽到它，说明今天的 agent 会把“我多写了测试”误认为“我提高了质量”。它会在没有新增信心的地方新增执行时间，在没有覆盖风险的地方覆盖你的耐心。
