# Model notes

Reference for the models wired into pi (`pi/models.json` + `pi/settings.json`).
Prices are USD per million tokens unless noted. These shift quickly — re-check
`ollama.com/pricing` and `openrouter.ai/models` before trusting a number.

## Ollama Cloud (`:cloud` models)

Cloud-hosted open models via `ollama signin`. The full catalog is only 16 models;
`ollama show <model>:cloud` confirms context/quantization. Prices below are list
(on-peak); off-peak (outside 12:00–18:00 UTC weekdays) is ~half on many models.

| Model | Input / Cached / Output | Context | Notes |
|---|---|---|---|
| `deepseek-v4-pro` | 1.32 / 0.044 / 3.96 | 1M | Frontier MoE. Current default in pi. Strong generalist + code. |
| `kimi-k3` | 3.00 / 0.30 / 15.00 | 1M | Most capable multimodal agent model here, but priciest. |
| `kimi-k2.7-code` | 0.95 / 0.19 / 4.00 | 256K | Coding-focused agent model, vision in. |
| `glm-5.3` | 1.40 / 0.26 / 4.40 | 1M | Long-horizon agent tasks. |
| `minimax-m3` | 0.60 / 0.12 / 2.40 | 1M | Cheap frontier-tier coding/agent, vision in. |
| `mistral-large-3` | 0.50 / — / 1.50 | 256K | Solid all-rounder, no cache-read discount. |
| `deepseek-v4.1-flash` | 0.30 / 0.006 / 1.20 | 1M | Fast/cheap DeepSeek, vision in. |
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
  coding `qwen3-coder-*`, and cheap open-weight `qwen3.8-27b`.
- **Local GGUF**: `ollama pull qwen3.8:27b` etc. Needs RAM/VRAM; free after download.

OpenRouter picks wired into pi (prices per-token from the API, shown per-M):

| Model | Input / Output | Context | Use |
|---|---|---|---|
| `qwen3.8-max-0902` | ~2.00 / 6.00 | 1M | Frontier. Strongest Qwen at a sane price. `-max-prime` is better but ~2x cost. |
| `qwen3-coder-plus` | ~0.65 / 3.25 | 1M | Coding/agent specialist. |
| `qwen3.8-27b` | ~0.42 / 3.00 | 1M | Open-weight workhorse, cheap, also runs local. |
| `qwen3.8-flash` | ~0.15 / 0.47 | 1M | Fast/cheap for routine edits and lookups. |

## Choosing

- **Default / hard problems** — `deepseek-v4-pro` (cloud) or `qwen3.8-max` (OpenRouter).
- **Coding agent** — `kimi-k2.7-code` or `qwen3-coder-plus`.
- **Cheap + capable** — `minimax-m3` or `qwen3.8-27b`.
- **Fast/cheap fill-in** — `deepseek-v4.1-flash`, `glm-5.3-flash`, `qwen3.8-flash`.
- **Vision** — `kimi-k3`, `kimi-k2.7-code`, `minimax-m3` (cloud); `qwen3-vl-*` (OpenRouter/local).
- **Fully offline** — `ornith-agent`, `gpt-oss-agent` (local GGUF, see `pi/ollama/`).

"Strongest" ranking here is practical (capability per dollar for agent/coding), not
benchmarks. Frontier models rotate fast; the `-flash`/`-plus`/`-max` tier names are a
better signal than any static ranking.
