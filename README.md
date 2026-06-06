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

### 3. 检索查询生成（Retrieve）

从角色记忆图谱自动生成结构化的检索查询，用于评估检索系统的质量：

- 自动生成模拟真实用户提问的检索查询（≥60 个）
- 包含语义查询（Semantic）和情境查询（Situation）两种变体
- 查询集合（Query Set）提供同一意图的多种口语化表达变体
- 每个查询自动标注期望命中的记忆节点（must_include / may_include）
- 支持 `--tendency` 参数控制生成倾向

## 快速开始

### 环境要求

- Rust 1.75+ / Cargo
- OpenAI API Key 或兼容的 API Base

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

```bash
soul_scraper --model gpt-4o --scrape "https://zh.moegirl.org.cn/十六夜咲夜" --output result.md
```

### 提取记忆图谱

```bash
# 从文件读取
soul_scraper --model gpt-4o --extract result.md --output memory.json

# 从标准输入读取
type result.md | cargo run -- --model gpt-4o --extract - --output memory.json

# 直接传入文本
soul_scraper --model gpt-4o --extract "十六夜咲夜是红魔馆的女仆长..." --output memory.json
```

### 生成检索查询

```bash
# 基础用法
soul_scraper --model gpt-4o --retrieve --query memory.json --output queries.json

# 带生成倾向
soul_scraper --model gpt-4o --retrieve --query memory.json --output queries.json --tendency "侧重人物关系查询，多使用Semantic变体"
```

## 命令行参数

| 参数 | 说明 |
|------|------|
| `--model <MODEL>` | 使用的 LLM 模型名称（如 `gpt-4o`）|
| `--scrape <URL>` | 从指定 URL 抓取角色信息 |
| `--extract <INPUT>` | 从文件路径、内容字符串或 `-`（stdin）提取记忆 |
| `--retrieve` | 检索查询生成模式 |
| `--query <INPUT>` | 检索查询的输入（记忆图谱 JSON 文件路径或内容）|
| `--tendency <STR>` | 查询生成倾向（可选） |
| `-o, --output <PATH>` | 输出文件路径，`-` 表示 stdout |
| `--api-base <URL>` | OpenAI 兼容 API 地址（可选）|



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

### 检索查询输出（JSON）

```json
{
  "queries": [
    {
      "priority": 9,
      "tags": ["人物"],
      "expected": {
        "must_include": ["sem_bronya"],
        "may_include": []
      },
      "variant": {
        "variant_kind": "Semantic",
        "units": [
          {
            "concept_identifier": "她的姐姐大人",
            "description": "那个在孤儿院一直保护她的银发女生"
          }
        ]
      }
    }
  ],
  "query_sets": [
    {
      "set_id": "set_bronya",
      "description": "查询希儿最重要的人",
      "queries": [
        {
          "priority": 9,
          "tags": ["人物"],
          "expected": { "must_include": ["sem_bronya"], "may_include": [] },
          "variant": {
            "variant_kind": "Semantic",
            "units": [{ "concept_identifier": "她的姐姐大人", "description": "那个总是保护她的银发女生" }]
          }
        }
      ]
    }
  ]
}
```

## 配套项目

- [SoulMem](https://github.com/dynamder/SoulMem) - 角色扮演记忆系统（上游数据消费方）

## License

MIT
