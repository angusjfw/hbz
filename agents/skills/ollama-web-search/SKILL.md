---
name: ollama-web-search
description: "Search the web via Ollama Cloud's web_search API. Use when you need current information, facts, or sources from the internet."
---

# Ollama Web Search

Queries Ollama Cloud's `web_search` API and returns search results with titles, URLs, and snippets.

## Setup

Create an API key at https://ollama.com/settings/keys, then either:

- Set the environment variable: `export OLLAMA_API_KEY=ollama_...`
- Or add it to `~/.pi/agent/auth.json`:

```json
{
  "ollama": {
    "type": "api_key",
    "key": "ollama_..."
  }
}
```

## Usage

Run from this skill directory:

```bash
node search.mjs "<query>" [--max-results N] [--json]
```

Examples:

```bash
node search.mjs "latest python release"
node search.mjs "vite 7 breaking changes" --max-results 10
node search.mjs "ollama new engine" --json
```

## Output

Returns up to `max_results` results (default 5, max 10), each with:

- `title`: page title
- `url`: full canonical URL
- `content`: relevant snippet

## See also

- `native-web-search` for Anthropic / OpenAI Codex native search (requires separate credentials).
- `web-browser` for browsing, screenshots, and page interaction via Chrome CDP.
