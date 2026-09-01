---
name: user-role
description: 用户是 C++ 标准中译本的维护者，正在用 Rust 开发 redoc 工具链
metadata: 
  node_type: memory
  type: user
  originSessionId: 922d0c21-29f8-40df-affe-430457219372
---

用户维护 **loc-iso14882**：自定义 redoc 格式的多语言（英/中）C++ 标准翻译项目（3499 个 .redoc 文件，对齐 N5054）。

相关三个工作空间（关系：draft 是官方 LaTeX 源 → loc-iso14882 用 redoc 逐条翻译 → redoc 仓库提供工具支持）：
- `c:\Users\yinan_li\source\draft`：cplusplus/draft 官方 LaTeX 源码，作转换/校对参照
- `c:\Users\yinan_li\source\loc-iso14882`：redoc 双语标准，**工具链的黄金测试语料**
- `c:\Users\yinan_li\source\redoc`：redoc 仓库（git 远程 lynnboy/redoc），含 VSCode 扩展 + Rust 工具链

技术能力：精通 C++ 标准，熟悉 Rust 与多种语言；机器有 rustc 1.93 nightly、MSVC 2022 + CMake。
