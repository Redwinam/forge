---
title: 用 Tauri 做一个笔记应用
date: 2024-01-01
tags: [tauri, rust, vue]
categories:
  - 开发
# 草稿状态
draft: false
---
# 用 Tauri 做一个笔记应用

> 写在前面：这是一篇示例文章。

## 背景

最近在折腾 **Tauri 2**，顺手做了一个 Markdown 编辑器。主要功能：

1. 文件树浏览
2. 所见即所得编辑
3. 图片上传到 COS

::: tip 提示  
VitePress 的自定义容器语法。  
:::

## 实现

核心代码如下：

```rust
#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}
```


| 平台      | 状态  |
| ------- | --- |
| macOS   | ✅   |
| Windows | 🚧  |


![截图](https://cdn.example.com/press/images/screenshot.png)

- [x] 基本编辑
- [ ] 全文搜索

[[toc]]

Cmd + S 保存。转义字符：\*不是斜体\*，1. 不是列表。

脚注引用[^1](脚注内容。)。