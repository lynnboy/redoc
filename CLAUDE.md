# CLAUDE.md

## 项目定位

redoc 仓库包含两大部分：

1. **VSCode 扩展**（根目录，`package.json` + `syntaxes/`）——为 `.redoc` 文件提供语法高亮（TextMate 正则）
2. **Rust 工具链**（`crates/`，cargo workspace）——redoc 格式的解析、校验、转换、编译工具

redoc 格式的权威规范在 `spec/spec.md`。**实现与语料冲突时以 spec 为准**；若发现
spec 未覆盖的真实用法，先更新 spec 再改代码。

## 相关仓库（工作区外）

- `loc-iso14882`：redoc 格式的多语言 C++ 标准源（3499 个 `.redoc` 文件），是**黄金测试语料**
- `draft`：C++ 标准官方 LaTeX 源码，是转换/校对的目标参照

## 构建与测试

```bash
cargo build          # 构建 workspace
cargo test           # 单元测试（crates/core）
cargo run -p redoc-cli -- parse <file>     # 解析并打印文档树
cargo run -p redoc-cli -- check <file|dir> # 解析+校验，报告诊断
```

## 语料即测试

`loc-iso14882` 的全部 `.redoc` 文件是回归测试金矿。改解析器后必须验证：

```bash
cargo run -p redoc-cli -- check C:\Users\yinan_li\source\loc-iso14882
```

关注错误数量变化：合理的改动应只减少误报，不应引入新的解析错误。

## 内存纪律（重要）

`redoc check` 遍历全语料时**禁止**把所有文件的解析树同时驻留内存——
曾因此导致 ~10GB 峰值。两遍流式模式（见 `crates/cli/src/main.rs` 的 `cmd_check`）：
第一遍只收集锚点索引（HashSet），第二遍逐文件解析-校验-立即丢弃。

## 架构

```
crates/core   tokenizer → parser → AST → validate（无依赖，纯核心）
crates/cli    redoc parse / redoc check 命令
spec/         格式规范 spec.md
syntaxes/     VSCode 扩展的 TextMate 语法（与 Rust 解析器独立，已知有偏差）
```

关键解析规则（详见 spec/spec.md）：

- 所有特殊内容在 `[...]` 内；`` `[ `` 等为转义；`[/ ... /]` 为注释
- `[` 后紧跟 `` ` `` 是 inline-code 标记（非转义）
- 区域标记（table/syntax/codeblock/div/list/rule/note/begin）必须有结束标记；
  **section 的结束标记可选**（到 EOF 自动闭合）
- 子语言区域（codeblock/math/formula/figure）内容原样捕获，不按 redoc 解析
- 代码块内 redoc 语义经 `[[redoc("...")]]` 属性或 `// [:en]...` 注释注入
