---
name: web-search
description: "Search the web using the active provider's native search capability. Auto-detects Ollama Cloud, Anthropic, or OpenAI Codex based on Pi settings and stored credentials."
---

# Web Search

Run a web search through whichever provider is currently active in Pi,
falling back to the first provider with usable credentials.

Supported credentials are environment variables and `~/.pi/agent/auth.json`
entries. You can force a specific backend with `--provider`.

## Supported backends

| Provider | Mechanism | Credential |
|---|---|---|
| `ollama` | Ollama Cloud `https://ollama.com/api/web_search` | `OLLAMA_API_KEY` env or `auth.json.ollama.key` |
| `anthropic` | Claude native `web_search_20250305` tool | `ANTHROPIC_API_KEY` env or `auth.json.anthropic.key` |
| `openai-codex` | ChatGPT Codex native `web_search` tool | `auth.json["openai-codex"]` OAuth entry |

## Usage

```bash
node search.mjs "<query>" [--provider ollama|anthropic|openai-codex] [--max-results N] [--json]
```

Examples:

```bash
node search.mjs "latest python release"
node search.mjs "vite 7 breaking changes" --provider ollama --max-results 10
node search.mjs "HTTP/3 browser support" --provider anthropic
node search.mjs "what is the best pi extension" --json
```

## How the provider is chosen

1. `--provider` flag if given.
2. `defaultProvider` from `~/.pi/agent/settings.json`.
3. `PI_PROVIDER` environment variable.
4. First provider with credentials available in `~/.pi/agent/auth.json` or environment variables.

## Output

- `ollama`: returns a list of results with `title`, `url`, and `content`.
- `anthropic` / `openai-codex`: returns a concise research summary with full source URLs.

Use `--json` to get a structured response including `provider`, `query`, and `result`.

## Setup

### Ollama Cloud

Create a web search API key at https://ollama.com/settings/keys, then either export it:

```bash
export OLLAMA_API_KEY=ollama_...
```

or add it to `~/.pi/agent/auth.json`:

```json
{
  "ollama": {
    "type": "api_key",
    "key": "ollama_..."
  }
}
```

### Anthropic

```bash
export ANTHROPIC_API_KEY=sk-ant-api03-...
```

or add to `~/.pi/agent/auth.json`:

```json
{
  "anthropic": {
    "type": "api_key",
    "key": "sk-ant-api03-..."
  }
}
```

### OpenAI Codex

Add the OAuth entry from your ChatGPT/Codex session to `~/.pi/agent/auth.json`:

```json
{
  "openai-codex": {
    "type": "oauth",
    "access": "eyJhbGciOiJ...",
    "refresh": "...",
    "expires": "2025-10-01T12:00:00.000Z",
    "accountId": "..."
  }
}
```

## How the provider is chosen

1. `--provider` flag if given.
2. `defaultProvider` from `~/.pi/agent/settings.json`.
3. `PI_PROVIDER` environment variable (set by Pi for shell-tool commands).
4. First provider with credentials available in `~/.pi/agent/auth.json` or
   environment variables.

## Notes

- This skill is independent of the model running the current Pi session. It can
  call Ollama Cloud search even when the active coding model is Kimi,
  DeepSeek, etc.
- Anthropic and OpenAI Codex implementations in `search.mjs` are adapted from
  `mitsuhiko/agent-stuff` (Apache-2.0). See `NOTICE`.
- For browsing, screenshots, and page interaction, use the `web-browser` skill
  instead.
