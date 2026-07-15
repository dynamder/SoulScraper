# Soul Scraper

[English Version](./README_en.md)

一个基于大语言模型（LLM）的虚拟角色信息抓取与提取工具，可从网页自动搜集角色资料并构建结构化的角色记忆图谱（SoulMem Graph），为角色扮演/角色模拟系统提供数据支撑。

## 核心功能

### 1. 角色抓取（Scrape）

从给定 URL 出发，由 LLM 驱动的 Agent 自动抓取并整合虚拟角色的详细信息：

- 从初始 URL 开始，自动识别并抓取相关链接页面
- 覆盖角色外貌、性格、经历、人物关系、行为习惯、价值观、能力等全部细节
- 输出为目标角色的**第一人称视角**研究报告

### 2. 记忆提取（Extract）— 两阶段管线

将角色研究报告转化为结构化的**记忆图谱**（Memory Graph），分两阶段执行：

**Phase 1 — 节点提取**：从文本中提取全部记忆节点（不含关联边）
- Semantic：人物、概念、事实等知识节点
- Situation：角色的具体/抽象经历
- Procedure：行为模式、习惯、条件反射
- 自动修复 LLM 输出中的 JSON 格式错误

**Phase 2 — 边生成**：基于已验证的节点列表，生成节点间的关联边
- Sem 边：概念间语义关联（如"侍奉""同伴"）
- Proc 边：情境触发的行为（概率标注）
- Situation 边：抽象情境→具体情境
- 自动移除非法边（Proc→Sit / Proc→Proc）
- 图结构质量验证（聚类系数、模块度、冗余度）

### 3. 检索查询生成（Retrieve）

从角色记忆图谱自动生成结构化的检索查询，用于评估检索系统的质量：

- 自动生成模拟真实用户提问的检索查询
- 包含语义查询（Semantic）和情境查询（Situation）两种变体
- 查询集合（Query Set）提供同一意图的多种口语化表达变体
- 每个查询自动标注期望命中的记忆节点（must_include / may_include）
- 支持 `--tendency` 参数控制生成倾向

### 4. 批量处理（Batch）

从 URL 列表文件批量处理多个角色：

- 支持 `name<TAB>url` 格式（角色名作为输出目录名）
- 断点续传：已有文件自动跳过
- 并发控制：`--parallel N`
- 原子日志输出：异步并发时不交叉混乱

## 快速开始

### 环境要求

- Rust 1.82+ / Cargo
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

### 单角色处理

```bash
# 抓取角色信息
soul_scraper --model gpt-4o --scrape "https://zh.moegirl.org.cn/十六夜咲夜" -o result.md

# 提取记忆图谱（两阶段自动化：节点→边→验证）
soul_scraper --model gpt-4o --extract result.md -o graph.json

# 生成检索查询
soul_scraper --model gpt-4o --retrieve --query graph.json -o questions.json
```

### 批量处理

URL 列表文件（每行一个，支持角色名）：

```txt
十六夜咲夜	https://zh.moegirl.org.cn/%E5%8D%81%E5%85%AD%E5%A4%9C%E5%92%B2%E5%A4%9C
博丽灵梦	https://zh.moegirl.org.cn/%E5%8D%9A%E4%B8%BD%E7%81%B5%E6%A2%A6
芙兰朵露	https://zh.moegirl.org.cn/%E8%8A%99%E5%85%B0%E6%9C%B5%E9%9C%B2
```

```bash
# 串行处理
soul_scraper --model gpt-4o --batch urls.txt --out-dir ./output

# 3 个并发
soul_scraper --model gpt-4o --batch urls.txt --out-dir ./output --parallel 3
```

批量输出目录结构：

```txt
output/
├── 十六夜咲夜/
│   ├── scrape.md              # 爬取报告
│   ├── graph.json             # 记忆图谱（GraphNodeRaw[]）
│   ├── graph_stats.json       # 图结构质量报告
│   └── question.json          # 检索测试用例（RetrQueryFileRaw）
└── 博丽灵梦/
    └── ...
```

## 命令行参数

| 参数 | 说明 |
|------|------|
| `--model <MODEL>` | 使用的 LLM 模型名称（如 `gpt-4o`）|
| `--scrape <URL>` | 从指定 URL 抓取角色信息 |
| `--extract <INPUT>` | 从文件路径、内容字符串或 `-`（stdin）提取记忆 |
| `--question <ARGS>` | 问题生成模式（含 `--retrieve`、`--consolidate`、`--forget`） |
| `--retrieve` | 检索查询生成模式 |
| `--query <INPUT>` | 检索查询的输入（记忆图谱 JSON 文件路径或内容）|
| `--tendency <STR>` | 查询生成倾向（可选） |
| `--batch <FILE>` | 批量模式：从文件中逐行读取 URL |
| `--out-dir <PATH>` | 批量模式输出根目录 |
| `--parallel <N>` | 批量模式并发数（默认 1） |
| `--name <STR>` | 测试套件名称（retrieve 模式）|
| `--description <STR>` | 测试套件描述 |
| `--graph-path <PATH>` | Graph JSON 相对路径 |
| `--similarity-threshold <F>` | 检索相似度阈值（默认 0.0）|
| `--max-results <N>` | 每次搜索最大返回数（默认 10）|
| `--test-k-values <LIST>` | 评估 k 值列表，如 `1,3,5` |
| `-o, --output <PATH>` | 输出文件路径，`-` 表示 stdout |
| `--api-base <URL>` | OpenAI 兼容 API 地址（可选）|

## 输出格式示例

### 提取输出（GraphNodeRaw[]）

```json
[
  {
    "id": "sem_self",
    "tags": ["自身", "角色"],
    "mem_type": {
      "Semantic": {
        "content": "十六夜咲夜",
        "aliases": ["十六夜咲夜", "咲夜", "红魔馆的女仆长", "完美而潇洒的女仆"],
        "concept_type": "Entity",
        "description": "我是红魔馆的女仆长，一个拥有操纵时间能力的人类..."
      }
    },
    "mem_links": [
      {
        "from": "sem_self",
        "to": "sem_remilia",
        "intensity": 0.9,
        "link_type": { "Sem": { "verb": "侍奉", "confidence": 0.9 } }
      }
    ]
  }
]
```

### 检索查询输出（RetrQueryFileRaw）

```json
{
  "name": "retr_sim_smoke_zh",
  "description": "向量相似性搜索冒烟测试",
  "graph_path": "graph.json",
  "config": {
    "similarity_threshold": 0.0,
    "max_results": 10,
    "test_k_values": [1, 3, 5]
  },
  "test_cases": [
    {
      "name": "query_0",
      "description": "原子查询 0",
      "sub_queries": [
        {
          "priority": 9,
          "tag": ["人物"],
          "variant": {
            "Semantic": [{
              "concept_identifier": "她的姐姐大人",
              "description": "那个在孤儿院一直保护她的银发女生"
            }]
          }
        }
      ],
      "expected_per_query": [{ "q": 0, "ranking": ["sem_self", "sem_bronya"] }],
      "expected_combined_ranking": ["sem_self", "sem_bronya"],
      "expected_actions": []
    }
  ]
}
```

### 图质量报告（graph_stats.json）

```json
{
  "node_count": 46,
  "edge_count": 147,
  "node_types": { "semantic": 22, "situation": 15, "procedure": 9 },
  "link_types": { "sem": 130, "proc": 7, "situation": 10 },
  "connected_components": 1,
  "largest_component": 46,
  "isolated_nodes": 0,
  "mst_forest_edges": 45,
  "global_redundancy": 2.27,
  "avg_clustering": 0.67,
  "community_modularity": 0.25,
  "intra_community_ratio": 1.0,
  "gini_coefficient": 0.41,
  "has_self_node": true,
  "self_description_ok": true,
  "illegal_edges": [],
  "is_clean": true,
  "is_structurally_valid": true,
  "failures": [],
  "warnings": []
}
```

## 图质量验证指标

管线在边生成后自动进行结构验证，不达标则重新生成：

| 指标 | 阈值 | 说明 |
|------|------|------|
| 聚类系数 | ≥ 0.20 | 局部稠密度：邻居间是否也互连 |
| 社区模块度 | ≥ 0.06 | 社区可划分性（< 0.15 时软警告） |
| 社区内边占比 | ≥ 0.60 | 边应集中于社区内部 |
| 全局冗余度 | ≥ 0.50 | 总边足够超过最小生成森林 |
| 分量冗余度 | ≥ 0.50 | 大型子图内部稠密 |
| 非法边 | = 0 | 无 Proc→Sit / Proc→Proc |
| 自身节点 | = true | 必须有 `sem_self` 且 description 含"我" |

## 诊断工具

项目包含几个诊断二进制：

```bash
# 检查 graph JSON 能否被 serde 解析
cargo run --bin diag

# 检查 question JSON 能否被 serde 解析
cargo run --bin diag_q

# 批量检查节点 JSON 解析错误
cargo run --bin check_nodes
```

## JSON 格式说明

所有枚举使用**外部标记**格式（externally tagged）：

| 枚举 | 示例 |
|------|------|
| MemoryType | `{"Semantic": {...}}` |
| MemoryLinkType | `{"Sem": {"verb": "侍奉", "confidence": 0.9}}` |
| ActionType | `"Speak"` / `{"Skill": {}}` / `"Think"` |
| SituationType | `{"SpecificSituation": {...}}` |
| AbstractSituation | `{"Location": {...}}` |
| RetrieveQueryVariant | `{"Semantic": [...]}` |

## 配套项目

- [SoulMem](https://github.com/dynamder/SoulMem) - 角色扮演记忆系统（上游数据消费方）

## License

MIT
