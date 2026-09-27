# Model notes

The **active set** (what shows in `/model` and `Ctrl+P`) is documented as comments
in `pi/models.json`, next to each model's cost/context. This file is the broader
catalog reference — the full Ollama Cloud catalog and OpenRouter Qwen pricing, plus
the reasoning behind the picks. Prices are USD per million tokens; re-check
`ollama.com/pricing` and `openrouter.ai/models` before trusting a number.

## Ollama Cloud (`:cloud` models)

Cloud-hosted open models via `ollama signin`. The full catalog is 16 models;
`ollama show <model>:cloud` confirms context/quantization. Prices are list (on-peak);
off-peak (outside 12:00–18:00 UTC weekdays) is ~half on many models. Models marked
**enabled** are wired into pi.

| Model | Input / Cached / Output | Context | Notes |
|---|---|---|---|
| `kimi-k2.7-code` | 0.95 / 0.19 / 4.00 | 256K | **enabled** — coding-focused agent model, vision in. Default. |
| `minimax-m3` | 0.60 / 0.12 / 2.40 | 1M | **enabled** — cheap frontier-tier coding/agent, vision in. |
| `deepseek-v4.1-flash` | 0.30 / 0.006 / 1.20 | 1M | **enabled** — fast/cheap coding fill-in, vision in. |
| `deepseek-v4-pro` | 1.32 / 0.044 / 3.96 | 1M | **enabled** — frontier MoE generalist, kept for hard reasoning. |
| `kimi-k3` | 3.00 / 0.30 / 15.00 | 1M | Most capable multimodal agent model, priciest. |
| `glm-5.3` | 1.40 / 0.26 / 4.40 | 1M | Long-horizon agent tasks. |
| `mistral-large-3` | 0.50 / — / 1.50 | 256K | Solid all-rounder, no cache-read discount. |
| `glm-5.3-flash` | 0.15 / 0.03 / 0.50 | 1M | Budget long-context. |
| `gpt-oss:120b` | 0.15 / 0.014 / 0.60 | 128K | Cheap OpenAI open model, reasoning. |
| `gpt-oss:20b` | 0.07 / 0.035 / 0.30 | 128K | Small/cheap, also runs locally. |
| `minimax-m2.7` | 0.30 / 0.06 / 1.20 | 196K | Mid-tier coding. |
| `gemma4:31b` | 0.14 / 0.05 / 0.40 | 256K | Small Google model, cheap. |
| `kimi-k2.6` | 0.95 / 0.16 / 4.00 | 256K | Predecessor to k2.7-code. |
| `glm-5.2` | 1.40 / 0.26 / 4.40 | 1M | Predecessor to glm-5.3. |
| `nemotron-3-nano/super/ultra` | 0.06–0.10 in | 256K | NVIDIA models, cheap; smallest are weak for agent work. |

## Qwen

Not on Ollama Cloud. Two routes:

- **OpenRouter** (preferred — already authenticated): frontier `qwen3.8-max`,
  coding `qwen3-coder-*`, and cheap open-weight models.
- **Local GGUF**: `ollama pull qwen3.8:27b` etc. Needs RAM/VRAM; free after download.

OpenRouter picks wired into pi (prices per-token from the API, shown per-M):

| Model | Input / Output | Context | Use |
|---|---|---|---|
| `qwen3-coder-plus` | ~0.65 / 3.25 | 1M | Coding/agent specialist. |
| `qwen3.8-flash` | ~0.15 / 0.47 | 1M | Fast/cheap for routine edits and lookups. |
| `qwen3.8-max-0902` | ~2.00 / 6.00 | 1M | Frontier, kept for hard reasoning. `-max-prime` is better but ~2x cost. |

## Choosing

- **Coding (default)** — `kimi-k2.7-code` (cloud).
- **Cheap + capable coding** — `minimax-m3` (cloud) or `qwen3-coder-plus` (OpenRouter).
- **Fast/cheap fill-in** — `deepseek-v4.1-flash` (cloud) or `qwen3.8-flash` (OpenRouter).
- **Hard reasoning, occasional** — `deepseek-v4-pro` (cloud) or `qwen3.8-max-0902` (OpenRouter).
- **Vision** — `kimi-k2.7-code`, `minimax-m3` (cloud); `qwen3-vl-*` (OpenRouter/local).
- **Fully offline** — `ornith-agent`, `gpt-oss-agent` (local GGUF, see `pi/ollama/`).

"Strongest" ranking here is practical (capability per dollar for agent/coding), not
benchmarks. Frontier models rotate fast; the `-flash`/`-plus`/`-max` tier names are a
better signal than any static ranking.
