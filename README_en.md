# Soul Scraper

[中文版本](./README.md)

An LLM-powered virtual character information scraper and extractor. It automatically collects character data from web pages and builds structured SoulMem graphs, providing data support for character role-playing/simulation systems.

## Features

### 1. Character Scraping (Scrape)

AI Agent-driven automatic collection of detailed virtual character information starting from a given URL:

- Starting from the initial URL, automatically identifies and scrapes relevant linked pages (max depth: 2)
- Covers all character details: appearance, personality, experiences, relationships, behavior patterns, values, abilities, etc.
- Outputs as a **first-person perspective** character research report

### 2. Memory Extraction (Extract)

Converts character research reports into structured **Memory Graphs**:

- **Semantic Memory**: Conceptual knowledge nodes (people, concepts, facts)
- **Situational Memory**: Character's specific/abstract experiences
- **Procedural Memory**: Behavior patterns, habits, conditioned reflexes
- Automatically fixes JSON format errors in LLM output

### 3. Retrieval Query Generation (Retrieve)

Automatically generates structured retrieval queries from character memory graphs for evaluating retrieval system quality:

- Auto-generates 60+ retrieval queries that simulate real user questions
- Includes both Semantic and Situation query variants
- Query Sets provide multiple colloquial expression variants for the same intent
- Each query is annotated with expected retrieval nodes (must_include / may_include)
- Supports `--tendency` flag to control generation bias

## Quick Start

### Requirements

- Rust 1.75+ / Cargo
- OpenAI API Key or compatible API Base

### Build

```bash
cargo build --release
```

### Configure API Key

```bash
# Linux/macOS
export SOUL_SCRAPER_KEY=your_api_key_here

# Windows PowerShell
$env:SOUL_SCRAPER_KEY="your_api_key_here"
```

### Scrape Character Info

Scrape character research report from a web URL:

```bash
soul_scraper --model gpt-4o --scrape "https://zh.moegirl.org.cn/十六夜咲夜" --output result.md
```

### Extract Memory Graph

Read character info from file or stdin, output JSON format memory graph:

```bash
# From file
soul_scraper --model gpt-4o --extract result.md --output memory.json

# From stdin
type result.md | cargo run -- --model gpt-4o --extract - --output memory.json

# Direct text input
soul_scraper --model gpt-4o --extract "Sakuya is the head maid of the Scarlet Devil Mansion..." --output memory.json
```

### Generate Retrieval Queries

Auto-generate retrieval queries from a memory graph JSON file:

```bash
# Basic usage
soul_scraper --model gpt-4o --retrieve --query memory.json --output queries.json

# With generation tendency
soul_scraper --model gpt-4o --retrieve --query memory.json --output queries.json --tendency "Focus on relationship queries, prefer Semantic variant"
```

## Command Line Options

| Option | Description |
|--------|-------------|
| `--model <MODEL>` | LLM model name (e.g., `gpt-4o`) |
| `--scrape <URL>` | Scrape character info from specified URL |
| `--extract <INPUT>` | Extract memory from file path, content string, or `-` (stdin) |
| `--retrieve` | Retrieval query generation mode |
| `--query <INPUT>` | Input for retrieval query generation (memory graph JSON file path or content) |
| `--tendency <STR>` | Query generation tendency (optional) |
| `-o, --output <PATH>` | Output file path, `-` for stdout |
| `--api-base <URL>` | OpenAI-compatible API base URL (optional) |



## Output Format Examples

### Scrape Output (Markdown)

```markdown
## Basic Information
- Gender: Female
- Age (apparent age): Forever 17
...

## Self Description

## Relationships

## Behavior Patterns

## Values
```

### Extract Output (JSON)

```json
{
  "graph": {
    "nodes": [
      {
        "node_id": "sem_1",
        "tags": ["character", "master"],
        "mem_type": {
          "mem_kind": "Semantic",
          "content": "Remilia Scarlet",
          "description": "The mistress of the Scarlet Devil Mansion..."
        }
      }
    ],
    "links": [...]
  },
  "summary": "I, Sakuya Izayoi, am the head maid of the Scarlet Devil Mansion..."
}
```

### Retrieval Query Output (JSON)

```json
{
  "queries": [
    {
      "priority": 9,
      "tags": ["character"],
      "expected": {
        "must_include": ["sem_bronya"],
        "may_include": []
      },
      "variant": {
        "variant_kind": "Semantic",
        "units": [
          {
            "concept_identifier": "her big sister",
            "description": "the silver-haired girl who always protected her at the orphanage"
          }
        ]
      }
    }
  ],
  "query_sets": [
    {
      "set_id": "set_bronya",
      "description": "Query about the most important person to Seele",
      "queries": [
        {
          "priority": 9,
          "tags": ["character"],
          "expected": { "must_include": ["sem_bronya"], "may_include": [] },
          "variant": {
            "variant_kind": "Semantic",
            "units": [{ "concept_identifier": "her big sister", "description": "the silver-haired girl who always protected her" }]
          }
        }
      ]
    }
  ]
}
```

## Related Projects

- [SoulMem](https://github.com/dynamder/SoulMem) - Character role-playing memory system (downstream data consumer)

## License

MIT
