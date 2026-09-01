---
name: project-redoc-toolchain
description: redoc 工具链（Rust）开发进度：骨架已提交，含解析器/校验器/规范/内存纪律
metadata: 
  node_type: memory
  type: project
  originSessionId: 922d0c21-29f8-40df-affe-430457219372
---

redoc 工具链（`c:\Users\yinan_li\source\redoc`，远程 lynnboy/redoc）已搭好 Rust 骨架并提交（commit 786c5c2，2026-09-01）：

- `crates/core`：tokenizer + parser + AST + validate（无依赖）；`crates/cli`：`redoc parse` / `redoc check`
- `spec/spec.md`：格式权威规范；`CLAUDE.md`：项目基准
- 技术栈：Rust（用户选定），TS 扩展保留在仓库根，未来经 LSP 集成
- 路线图：解析/校验（已起步）→ LaTeX ⇄ redoc 转换/校对 → redoc → HTML/PDF 编译

**Why:** 用户要为自己的格式建完整工具链；语料即测试，内存纪律是硬约束。

**How to apply:** 改动后跑 `cargo test` 与 corpus check（3498 文件）。**禁止**把全语料解析树同时驻留内存（曾致 10GB 峰值，已改为两遍流式）。实现与语料冲突时以 spec 为准。下一步方向（用户未定）：抽查剩余 1747 错误中哪些是工具误报 vs 语料真问题，或写 spec 细化，或继续扩展转换/编译。
