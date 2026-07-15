# Soul Scraper

[中文版本](./README.md)

An LLM-powered virtual character information scraper and extractor. It automatically collects character data from web pages and builds structured SoulMem graphs, providing data support for character role-playing/simulation systems.

## Features

### 1. Character Scraping (Scrape)

AI Agent-driven automatic collection of detailed virtual character information starting from a given URL:

- Starting from the initial URL, automatically identifies and scrapes relevant linked pages
- Covers all character details: appearance, personality, experiences, relationships, behavior patterns, values, abilities, etc.
- Outputs as a **first-person perspective** character research report

### 2. Memory Extraction (Extract) — Two-Phase Pipeline

Converts character research reports into structured **Memory Graphs** in two phases:

**Phase 1 — Node Extraction**: Extract all memory nodes (no edges)
- **Semantic**: Characters, concepts, facts
- **Situation**: Specific/abstract experiences
- **Procedure**: Behavior patterns, habits, conditioned reflexes
- Auto-fixes JSON format errors from LLM output

**Phase 2 — Edge Generation**: Generate edges between validated nodes
- **Sem** edges: Semantic associations (verb + confidence)
- **Proc** edges: Situation-triggered behaviors (probability indexed)
- **Situation** edges: Abstract-to-specific
- Auto-removes illegal edges (Proc→Sit, Proc→Proc)
- Graph quality validation (clustering, modularity, redundancy)

### 3. Retrieval Query Generation (Retrieve)

Generates structured retrieval queries from memory graphs for evaluating retrieval systems:

- Semantic and Situation query variants
- Query sets with multiple paraphrased variants
- Auto-annotates expected note matches (must_include / may_include)
- Supports `--tendency` parameter

### 4. Batch Processing (Batch)

Process multiple characters from a URL list file:

- Supports `name<TAB>url` format (names used as output directory names)
- Resume: skips existing output files
- Concurrency: `--parallel N`
- Atomic logging: no interleaving in concurrent mode

## Quick Start

### Prerequisites

- Rust 1.82+ / Cargo
- OpenAI API Key or compatible API Base

### Build

```bash
cargo build --release
```

### Set API Key

```bash
export SOUL_SCRAPER_KEY=your_api_key_here
```

### Single Character

```bash
# Scrape
soul_scraper --model gpt-4o --scrape "https://zh.moegirl.org.cn/十六夜咲夜" -o result.md

# Extract (two-phase: nodes → edges → validation)
soul_scraper --model gpt-4o --extract result.md -o graph.json

# Generate questions
soul_scraper --model gpt-4o --retrieve --query graph.json -o questions.json
```

### Batch Processing

URL list file format:

```txt
Sakuya	https://zh.moegirl.org.cn/%E5%8D%81%E5%85%AD%E5%A4%9C%E5%92%B2%E5%A4%9C
Reimu	https://zh.moegirl.org.cn/%E5%8D%9A%E4%B8%BD%E7%81%B5%E6%A2%A6
```

```bash
# Sequential
soul_scraper --model gpt-4o --batch urls.txt --out-dir ./output

# Parallel (3 concurrent)
soul_scraper --model gpt-4o --batch urls.txt --out-dir ./output --parallel 3
```

## CLI Parameters

| Parameter | Description |
|-----------|-------------|
| `--model <MODEL>` | LLM model name (e.g. `gpt-4o`) |
| `--scrape <URL>` | Scrape character from URL |
| `--extract <INPUT>` | Extract memory from file, string, or stdin |
| `--retrieve` | Retrieval query generation mode |
| `--query <INPUT>` | Input memory graph JSON |
| `--tendency <STR>` | Query generation tendency (optional) |
| `--batch <FILE>` | Batch mode: read URLs from file |
| `--out-dir <PATH>` | Batch output root directory |
| `--parallel <N>` | Batch concurrency (default 1) |
| `-o, --output <PATH>` | Output file path, `-` for stdout |
| `--api-base <URL>` | OpenAI compatible API base URL |

## Output Format

### Graph JSON (GraphNodeRaw[])

```json
[
  {
    "id": "sem_self",
    "tags": ["self", "character"],
    "mem_type": {
      "Semantic": {
        "content": "Sakuya Izayoi",
        "aliases": ["Sakuya", "Remilia's maid"],
        "concept_type": "Entity",
        "description": "I am the head maid of the Scarlet Devil Mansion..."
      }
    },
    "mem_links": [
      {
        "from": "sem_self",
        "to": "sem_remilia",
        "intensity": 0.9,
        "link_type": { "Sem": { "verb": "serves", "confidence": 0.9 } }
      }
    ]
  }
]
```

### Graph Stats (graph_stats.json)

```json
{
  "node_count": 46,
  "edge_count": 147,
  "global_redundancy": 2.27,
  "avg_clustering": 0.67,
  "community_modularity": 0.25,
  "is_structurally_valid": true,
  "illegal_edges": [],
  "failures": []
}
```

## License

MIT
