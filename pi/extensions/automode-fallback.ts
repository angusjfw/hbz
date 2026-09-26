/**
 * pi-automode takes one classifier model and blocks everything if it can't reach
 * it. The configured classifier is local (Ollama), which isn't always running, so
 * when Ollama is down at startup, point this session's classifier at a cheap
 * hosted model instead. Runs in the factory: pi-automode reads its env override
 * at session_start, after every extension has loaded.
 */

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

const OLLAMA = "http://127.0.0.1:11434/api/version";
const FALLBACK = "openrouter/deepseek/deepseek-v4-flash";

export default async function (_pi: ExtensionAPI) {
	if (process.env.PI_AUTOMODE_SETTINGS_JSON) return;
	const up = await fetch(OLLAMA, { signal: AbortSignal.timeout(500) }).then(
		(r) => r.ok,
		() => false,
	);
	if (!up) {
		process.env.PI_AUTOMODE_SETTINGS_JSON = JSON.stringify({ autoMode: { classifierModel: FALLBACK } });
	}
}
