# bmsrs

[English](README.md) | 简体中文

BMS 与 BMSON 谱面格式的 Rust 解析、转换与模拟播放工具包。

> 预发布状态：`0.1.0` 尚未发布，公开接口可能变动。

## 简介

bmsrs 处理两种谱面格式：

- BMS：文本格式，提供从原始字节到结构化模型的完整管道。
- BMSON：JSON 格式，支持 v0、v1、v2 类型定义与升版。

两条管道输出统一的 `Chart<T>` 中央 IR。渲染器、播放器与工具只需消费这一层。

项目使用 Rust 2024 edition，MSRV 为 1.95。

## 快速开始

`0.1.0` 尚未发布。当前请使用 git 依赖：

```toml
[dependencies]
bmsrs = { git = "https://github.com/brightmeows/bmsrs" }
```

完整的管道说明、crate 职责表、BMS/BMSON 示例与开发命令见英文主版
[README.md](README.md)；英文版是唯一事实来源，本页只保留中文导读。

## 许可证

Apache-2.0，见 [`LICENSE`](LICENSE)。
