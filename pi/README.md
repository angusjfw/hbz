# Pi setup

This directory configures the [Pi coding agent](https://pi.dev). It is
symlinked into `~/.pi/agent/` by `make pi` / `make ai`.

## Quick start on a new machine

1. Run `make ai` (or `make pi`) from the repo root.
2. Run `/login <provider>` inside Pi (Pi slash command) for whichever providers
   this machine uses.
3. If using Ollama Cloud web search, create a web-search API key at
   https://ollama.com/settings/keys and add it to `~/.pi/agent/auth.json`:

   ```json
   {
     "ollama": {
       "type": "api_key",
       "key": "ollama_..."
     }
   }
   ```

4. Check `pi/MODELS.md` for the current model picks and pricing.

## Web search

The shared `agents/skills/web-search` skill uses whichever provider is
available. It reads `~/.pi/agent/settings.json` (the `defaultProvider` field),
the `PI_PROVIDER` environment variable that Pi exposes to shell tools, and
`~/.pi/agent/auth.json` to pick a backend in that order, or you can force one
with `--provider ollama|anthropic|openai-codex`.

Supported backends and their credentials:

| Provider | Source |
|---|---|
| Ollama Cloud | `OLLAMA_API_KEY` env or `auth.json.ollama.key` |
| Anthropic | `ANTHROPIC_API_KEY` env or `auth.json.anthropic.key` |
| OpenAI Codex | `auth.json["openai-codex"]` OAuth entry |

Test it:

```bash
node ~/.pi/agent/skills/web-search/search.mjs "latest pi release notes"
```

## Web browser

Browsing goes through two MCP servers that `make browser` adds to
`mcp-adapter.json` (see `browser/`): `playwright` for headless, isolated work,
and `chrome-devtools` for logged-in sessions in your own Chrome.

## MCP servers

The `pi-mcp-adapter` package gives Pi MCP access through one `mcp` proxy tool;
servers start lazily on first use. The server list is machine-local, in
`~/.pi/agent/mcp-adapter.json` (not in this repo, since it holds work endpoints).
Stdio servers inherit tokens from the shell environment. OAuth servers log in
with `/mcp-auth <server>` and keep tokens in the OS keychain. Use
`settings.approveTools` globs to require confirmation for write tools.

## Files

- `settings.json.example` — baseline Pi settings (default provider, enabled models,
  packages, skills). Merged into the live `settings.json` on `make pi`.
- `models.json` — provider and model definitions, with per-model notes and costs.
- `MODELS.md` — broader catalog reference and reasoning for the picks.
- `agents/*.md` — subagent definitions.
- `extensions/*.ts` — local Pi extensions.
  - `stash.ts` — `ctrl+s` stashes the editor draft; it comes back after the
    next prompt is sent, or on `ctrl+s` with an empty editor.
  - `recap.ts` — `/recap [focus]` summarises the session in a panel, without
    adding to the context.
- `automode.json` — config for the `pi-automode` package.
- `ollama/*.Modelfile` — local agent model overrides with larger context windows.

## Changing defaults

- Shared defaults: edit `pi/settings.json.example` or `pi/models.json` in this
  repo, then run `make pi`.
- Machine-only changes: edit the live `pi/settings.json` (gitignored, linked as
  `~/.pi/agent/settings.json`), or let Pi write them. `make pi` does a three-way
  merge with `scripts/merge-settings.py`: keys you haven't changed follow the
  baseline, keys you have changed keep your value. Each run lists the local
  overrides and warns when the baseline changed a key you also changed.
- Pi only starts on the default model if it is in `enabledModels`; otherwise it
  takes the first entry. Change both together.
