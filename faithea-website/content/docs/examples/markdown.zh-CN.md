---
title: Markdown 展示页
description: 用于测试 Markdown 元素样式的页面。
---

这不是框架文档，而是内容系统的视觉测试台。

## 代码与命令

行内代码类似 `cargo add something`，代码块则会获得语法高亮：

```rust
#[derive(Debug)]
struct TinySignal {
    label: &'static str,
    ready: bool,
}
```

## 列表与任务

- 普通列表保持清晰易读。
- 链接可以返回 [文档首页](../index.md)。
- 本地资源可以与 Markdown 放在一起。

- [x] 扫描 Markdown 文件
- [x] 构建静态页面
- [ ] 以后再编写真正的框架文档

## 表格

| 功能 | 构建时 | 浏览器运行时 |
| --- | ---: | ---: |
| Markdown 解析 | 是 | 否 |
| 目录扫描 | 是 | 否 |
| 客户端导航 | 否 | 是 |

## 本地图片

![装饰性信号卡片](../assets/example.svg)

### 更小的标题

右侧文章目录同时包含二级与三级标题。
