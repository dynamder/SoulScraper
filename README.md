# Soul Scraper

[English Version](./README_en.md)

一个基于大语言模型（LLM）的虚拟角色信息抓取与提取工具，可从网页自动搜集角色资料并构建结构化的角色记忆图谱（SoulMem Graph），为角色扮演/角色模拟系统提供数据支撑。

## 核心功能

### 1. 角色抓取（Scrape）

从给定 URL 出发，由 LLM 驱动的 Agent 自动抓取并整合虚拟角色的详细信息：

- 从初始 URL 开始，自动识别并抓取相关链接页面（最大跳转深度 2 层）
- 覆盖角色外貌、性格、经历、人物关系、行为习惯、价值观、能力等全部细节
- 输出为目标角色的**第一人称视角**研究报告

### 2. 记忆提取（Extract）

将角色研究报告转化为结构化的**记忆图谱**（Memory Graph）：

- **语义记忆**：人物、概念、事实等知识节点
- **情境记忆**：角色的具体/抽象经历
- **程序性记忆**：行为模式、习惯、条件反射
- 自动修复 LLM 输出中的 JSON 格式错误

## 快速开始

### 环境要求

- Rust 1.75+ / Cargo
- OpenAI API Key 或兼容的 API Base（如硅基流动等）

### 构建

```bash
cargo build --release
```

### 配置 API Key

```bash
# Linux/macOS
export SOUL_SCRAPER_KEY=your_api_key_here

# Windows PowerShell
$env:SOUL_SCRAPER_KEY="your_api_key_here"
```

### 抓取角色信息

从网页 URL 抓取角色研究报告：

```bash
soul_scraper --model gpt-4o --scrape "https://zh.moegirl.org.cn/十六夜咲夜" --output result.md
```

### 提取记忆图谱

从文件或标准输入读取角色信息，输出 JSON 格式的记忆图谱：

```bash
# 从文件读取
soul_scraper --model gpt-4o --extract result.md --output memory.json

# 从标准输入读取
type result.md | cargo run -- --model gpt-4o --extract - --output memory.json

# 直接传入文本
soul_scraper --model gpt-4o --extract "十六夜咲夜是红魔馆的女仆长..." --output memory.json
```

## 命令行参数

| 参数 | 说明 |
|------|------|
| `--model <MODEL>` | 使用的 LLM 模型名称（如 `gpt-4o`）|
| `--scrape <URL>` | 从指定 URL 抓取角色信息 |
| `--extract <INPUT>` | 从文件路径、内容字符串或 `-`（stdin）提取记忆 |
| `--question <INPUT>` | 问答模式（暂未实现）|
| `-o, --output <PATH>` | 输出文件路径，`-` 表示 stdout |
| `--api-base <URL>` | OpenAI 兼容 API 地址（可选）|

## 项目结构

```
soul_scraper/
├── src/
│   ├── main.rs                 # CLI 入口与参数解析
│   ├── scraper.rs             # 角色抓取模块（LLM Agent + Web Fetcher）
│   ├── extractor.rs           # 记忆提取模块
│   ├── io_src.rs              # 输入输出源抽象
│   ├── data_model/
│   │   ├── extractor.rs       # 提取输出的数据结构
│   │   └── soul_mem/          # 记忆系统数据模型
│   │       ├── sem.rs         # 语义记忆
│   │       ├── sit.rs         # 情境记忆
│   │       └── proc.rs        # 程序性记忆
│   └── prompt_template/
│       ├── scraper_system     # 抓取 Agent 系统提示
│       ├── extractor_system   # 提取系统提示
│       └── extractor_fix_system # JSON 修复系统提示
└── test_output/                # 测试输出示例
```

## 输出格式示例

### 抓取输出（Markdown）

```markdown
## 基本信息
- 性别：女
- 年龄（或外表年龄）：永远17岁
...

## 自我描述

## 人物关系

## 行为习惯

## 价值观
```

### 提取输出（JSON）

```json
{
  "graph": {
    "nodes": [
      {
        "node_id": "sem_1",
        "tags": ["人物", "主人"],
        "mem_type": {
          "mem_kind": "Semantic",
          "content": "蕾米莉亚·斯卡雷特",
          "description": "红魔馆的主人..."
        }
      }
    ],
    "links": [...]
  },
  "summary": "我，十六夜咲夜，是红魔馆的女仆长..."
}
```

## 技术特点

- **完全开源**：基于 Rust 语言，高性能、易于集成
- **LLM 驱动**：使用 OpenAI API 或兼容的第三方 API
- **结构化输出**：输出符合 JsonSchema 的标准化数据
- **自动容错**：内置 JSON 修复机制，应对 LLM 输出格式问题
- **灵活输入输出**：支持文件、字符串、标准输入/输出

## 配套项目

- [SoulMem](https://github.com/dynamder/SoulMem) - 角色扮演记忆系统（上游数据消费方）

## License

MIT
