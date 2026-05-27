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

## Quick Start

### Requirements

- Rust 1.75+ / Cargo
- OpenAI API Key or compatible API Base (e.g., SiliconFlow)

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

## Command Line Options

| Option | Description |
|--------|-------------|
| `--model <MODEL>` | LLM model name (e.g., `gpt-4o`) |
| `--scrape <URL>` | Scrape character info from specified URL |
| `--extract <INPUT>` | Extract memory from file path, content string, or `-` (stdin) |
| `--question <INPUT>` | Question answering mode (not yet implemented) |
| `-o, --output <PATH>` | Output file path, `-` for stdout |
| `--api-base <URL>` | OpenAI-compatible API base URL (optional) |

## Project Structure

```
soul_scraper/
├── src/
│   ├── main.rs                 # CLI entry and argument parsing
│   ├── scraper.rs              # Character scraping module (LLM Agent + Web Fetcher)
│   ├── extractor.rs            # Memory extraction module
│   ├── io_src.rs              # Input/Output source abstraction
│   ├── data_model/
│   │   ├── extractor.rs        # Extraction output data structures
│   │   └── soul_mem/           # Memory system data model
│   │       ├── sem.rs          # Semantic memory
│   │       ├── sit.rs          # Situational memory
│   │       └── proc.rs         # Procedural memory
│   └── prompt_template/
│       ├── scraper_system      # Scrape Agent system prompt
│       ├── extractor_system    # Extraction system prompt
│       └── extractor_fix_system # JSON fix system prompt
└── test_output/                # Test output examples
```

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

## Technical Features

- **Fully Open Source**: Built with Rust for high performance and easy integration
- **LLM-Powered**: Uses OpenAI API or compatible third-party APIs
- **Structured Output**: Outputs standardized data conforming to JsonSchema
- **Auto Error Recovery**: Built-in JSON fix mechanism for LLM output format issues
- **Flexible I/O**: Supports file, string, stdin/stdout

## Related Projects

- [SoulMem](https://github.com/dynamder/SoulMem) - Character role-playing memory system (downstream data consumer)

## License

MIT
