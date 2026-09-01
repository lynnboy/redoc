# redoc 格式规范

Redoc 是 loc-iso14882 项目用于承载 C++ 标准双语（英文/中文）翻译的标记语言。
**所有特殊内容都在 `[` `]` 内**，其余文本按原样处理。

本规范是 `redoc-core` 解析器的权威定义。实现与语料（`loc-iso14882`）有冲突时，
以本规范为准，语料中的偏差应作为校验器的诊断报告而非修改本规范。

---

## 1. 词法

### 1.1 转义

反引号转义产生字面字符：

| 转义 | 结果 |
|-|-|
| `` `[ `` | `[` |
| `` `] `` | `]` |
| ``` `` ``` | `` ` `` |
| `` `, `` | `,` |

歧义规则：`[` 后紧跟 `` ` `` 是 **inline-code 标记**（见 §3.7），该 `` ` `` 是标记名而非转义。

### 1.2 注释

`[/` 开始，`/]` 结束，可跨行，内容不解析。

### 1.3 空白与换行

缩进仅为排版习惯，不具语义。块结构由标记本身界定，不依赖缩进。

---

## 2. 标记（Tag）结构

```
[ 名字 :子类型 @属性=值 #锚点 内容 ]
```

| 部分 | 语法 | 说明 |
|-|-|-|
| 名字 | 标记性字符 或 `[A-Za-z_][A-Za-z0-9_]*` | 见 §3、§4 |
| 子类型 | `:` + `[A-Za-z0-9_~]+` | 如 `section:chapter`、`[:en]`、`[|:descriptive]` |
| 属性 | `@名` 或 `@名=值`，可多个 | 值为非空白、不含 `@#` 的字符串 |
| 锚点 | `#` + `[-A-Za-z0-9.:$]+` | 全局唯一，可被 `[#锚点]` 引用 |
| 内容 | 任意节点（文本/嵌套标记） | 到配对的 `]` 为止，可嵌套 |

内容内的 `]` 必须与内层 `[` 配对，括号匹配由解析器栈式处理。

### 2.1 结束标记（区域关闭）

两种风格，语义等价：

- `[名字:end]`：`[table:end]`、`[section:end]`、`[codeblock:end]`
- `[end:名字]`：`[end:note]`、`[end:example]`

### 2.2 区域（Region）与叶子（Leaf）

- **区域**：有开始标记，内容为开始标记内的节点；其**正文（body）**为开始标记
  与结束标记之间的兄弟节点。结束标记**必需**（除 section，见 §3.1）。
- **叶子**：无 body，仅开始标记括号内的内容。

---

## 3. 块级标记

### 3.1 文档与章节

| 标记 | 结束 | 说明 |
|-|-|-|
| `[document ...]` | 无（到 EOF） | 文档根，仅 `std.redoc` |
| `[section#id]`、`[section:chapter#id]`、`[section:ref#id]` | `[section:end]` **或** EOF | 结束标记可选；子类型有 `chapter`（章）、`ref`（参考文献区）、`back` 等 |
| `[para]`、`[para:~]` | 无 | 段落；`[:en]`/`[:zh_CN]` 行为其后续兄弟 |

### 3.2 描述区与列表

| 标记 | 结束 | 说明 |
|-|-|-|
| `[div:description]` | `[div:end]` | 图书馆条款的语义描述区 |
| `[list]`、`[list:ol@ndot]`、`[list:dl]` | `[list:end]` | 列表；`item` 为子项 |
| `[item#id]`、`[item:throws]` 等 | 无 | 列表项；子类型可为 `throws`/`returns`/`mandates` 等 |

### 3.3 语法产生式

```
[syntax]
[rule 名字 [:zh_CN] 中文名]
    [| 分支1 ]
    [| 分支2 ]
[rule:end]
[syntax:end]
```

| 标记 | 结束 | 说明 |
|-|-|-|
| `[syntax]`、`[syntax:explanation]` | `[syntax:end]` | 语法块 |
| `[rule 名字 ...]` | `[rule:end]` | 文法规则 |
| `[| ...]` | — | 产生式分支；`[|:descriptive ...]` 为描述性分支 |
| `[~名字]`、`[~:opt 名字]` | — | 分支内的语法非终结符（见 §4.2） |

### 3.4 代码块（子语言）

```
[codeblock:declaration]
... C++ 代码 ...
[codeblock:end]
```

子类型：`declaration`（声明）、`synopsis`（库摘要）、`notation`（记号）、
`literal`、`output`。无子类型即普通代码块。

**内容为原样捕获**，不按 redoc 解析（`[` 不触发标记），代码中的 redoc 语义
通过两种嵌入通道表达（见 §5）。

### 3.5 表格

```
[table:grid#id ...]       或  [table:listing@shape=?x1@fill=column#id ...]
[table:end]
```

- 表头单元格在开始标记的括号内：`[|@headerspan=2 内容]`
- 正文（body）为行：`[|@rowspan=4 单元格]`，空单元格 `[|]`
- 行分隔符：`[-]`

属性：`headerspan`、`rowspan`、`code`（代码列）等。

### 3.6 数学与图形（子语言）

| 标记 | 结束 | 说明 |
|-|-|-|
| `[math]`、`[math:aligned]` | `[math:end]` | LaTeX 数学，无编号 |
| `[formula#id]`、`[formula:aligned@...]` | `[formula:end]` | LaTeX 数学，有编号锚点 |
| `[figure:dot#id ...]` | `[figure:end]` | Graphviz dot 图形 |

内容均**原样捕获**。

### 3.7 内联代码

```
[`内容]                普通内联代码
[`:key signed]         关键词（keyword）
[`:c concept-name]     概念名（concept）
[`:opt 名字]           可选项（语法中）
[`@def 名字]           定义为某术语
[`@lib 头文件]         库名
```

子类型：`opt`、`key`、`c`、`cname`、`m`。属性：`@def`、`@lib`。
内容可嵌套其它内联代码标记。

### 3.8 标注、示例、脚注

| 标记 | 结束 | 说明 |
|-|-|-|
| `[begin:note]` | `[end:note]` | 注 |
| `[begin:example]` | `[end:example]` | 示例 |
| `[note:foot#id]` | `[note:end]` | 脚注，`[#:fn#id]` 引用 |

---

## 4. 内联标记

### 4.1 语言标记（langtag）

```
[:en]       [:zh_CN]       [:]
```

段落中 `[:en]` 与 `[:zh_CN]` 交替成对出现，后随的文本为其内容（兄弟节点）。
校验器检查二者数量平衡。

### 4.2 语法术语

| 标记 | 说明 |
|-|-|
| `[~名字]` | 语法非终结符，如 `[~identifier]` |
| `[~:opt 名字]` | 可选的语法项 |
| `[~@fake italic]` | 带排版属性 |

### 4.3 语义标记

| 标记 | 说明 |
|-|-|
| `[*名字]`、`[*:c 名字]` | 仅示明标识符（exposition-only identifier） |
| `[^名字]`、`[^:oc 概念]` | 占位符（placeholder），`oc` 为旧概念 |
| `[$名字]` | 数学变量（LaTeX 片段） |
| `[+术语]`、`[+:adj ...]`、`[+:% ...]` | 术语定义点 |

### 4.4 宏

```
[=名字]    [=名字(参数)]
```

文本宏，如 `[=Cpp]`、`[=lq]`/`[=rq]`（引号）、`[=']`、`[=--]`（破折号）、
`[=expos]`、`[=seebelow]`、`[=unspec]`、`[=cv]`、`[=range(first,last)]`。
完整宏集合以 `redoc-core` 的白名单为准。

### 4.5 交叉引用

```
[#锚点]           引用（`[section#锚点]`、`[item#锚点]` 等）
[#:note 锚点]     [#:tab]  [#:fig]  [#:fn]  [#:cite]  [#:eq]
[#@super 锚点]    指向外围章节
```

引用目标必须存在且锚点全局唯一。

### 4.6 索引

```
[%索引项]             主索引
[%索引项[!子项]]       带子索引
[!子项]               子索引
[@...]                索引-as（见 [%:ref@removed ...]）
```

### 4.7 求值标记（eval）

```
[?名字 @属性=值 @属性2 #锚点 内容]
```

名字（30+ 种，白名单见 `redoc-core`）按功能分：

| 类别 | 例子 |
|-|-|
| 库 | `[?libheader@ref cstdlib]`、`[?libhreader ...]` |
| 实现定义 | `[?impldef ...]`、`[?impdefx ...]`、`[?impldefrootname ...]` |
| 未定义行为 | `[?ubdef ...]`、`[?ub@desc ...]` |
| 无须诊断 | `[?ifndr#锚点]`、`[?ifndr@desc#锚点]` |
| 交叉参考 | `[?see ...]`、`[?also ...]`、`[?termref ...]`、`[?xrefc ...]`、`[?ref ...]` |
| 复杂度/语义 | `[?bigoh ...]`、`[?rationale ...]`、`[?effect ...]`、`[?change ...]`、`[?difficulty ...]` |
| 索引 | `[?indexordmem ...]`、`[?indexunordmem ...]`、`[?indexcont ...]`、`[?indexcond ...]` |

### 4.8 其它

| 标记 | 说明 |
|-|-|
| `[url https://...]` | URL |
| `[cite 标题]` | 引文 |
| `[br]` | 换行 |
| `[" ...]` | 引述 |
| `[&:em 内容]`、`[&:ucode XXXX]` | span：强调、Unicode 码点等 |
| `[!:mark ...]` | 代码块内标记注释 |

---

## 5. 代码块内的 redoc 嵌入

代码块内容是 C++ 子语言，redoc 语义通过两种通道注入：

### 5.1 `[[redoc("...")]]` 属性嵌入（零宽，作用于后续记号）

```cpp
template<[[redoc("`:c>")]]forward_iterator I, ...>   // 概念名
class [[redoc("*>")]]flipped { ... };                 // 仅示明标识符
void qsort(void*, size_t, [[redoc("^>")]]c-compare-pred*);  // 占位符
```

`>` 表示作用于后续记号，`<` 表示作用于前面记号。组合形式：
`` `:c> ``、`*>`、`^>`、`*:c>`、`` `:m< ``、`` `@def@lib< ``、`[=seebelow]` 等。

`[[redoc(...)]]` 是合法 C++ 属性，代码块可被 C++ 工具直接解析。

### 5.2 行注释内嵌双语

```cpp
extern void f();   // [:en] internal linkage [:zh_CN] 内部连接
```

---

## 6. 校验规则

解析器保证结构正确性（括号配对、区域闭合、转义）；`redoc-core::validate`
增加语义检查：

1. 锚点全局唯一，重复报错
2. `[#引用]` 必须解析到存在的锚点（跨文件由 CLI 汇总裁决）
3. `[:en]` / `[:zh_CN]` 数量平衡（警告）
4. 标记名在已知集合内，未知报警告（格式在演进，不报错）
5. `codeblock` 子类型限于 `declaration`/`synopsis`/`notation`/`literal`/`output`

---

## 7. 文件组织

- `std.redoc`：文档头（`[document ...]`、`[attribute#x [value ...]`）+ `[include ...]`
- 每章/子条款一个 `.redoc` 文件，一个顶层 `[section]`
- `[include 名字]` 无扩展名，解析为 `名字.redoc`；构成文档树，不得成环
- 章节目录：`lex/`、`expr/`、`stmt/`、`basic/`、`classes/`、`templates/`、
  `overloading/`、`algorithms/`（每算法一文件）等
- `ifndr/`：`[?ifndr#锚点]` 引用的"非良构无须诊断"规则定义
