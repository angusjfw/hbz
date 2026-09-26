/**
 * When pi finishes a run in a tmux pane you aren't looking at, flash a tmux
 * message naming the window, with the start of its last reply.
 */

import { execFile } from "node:child_process";

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

const MAX_BODY = 80;

const tmux = (args: string[]) =>
	new Promise<string>((resolve) =>
		execFile("tmux", args, { timeout: 2000 }, (err, stdout) => resolve(err ? "" : stdout.trim())),
	);

const lastAssistantText = (messages: Array<{ role?: string; content?: unknown }>): string => {
	for (let i = messages.length - 1; i >= 0; i--) {
		const m = messages[i];
		if (m?.role !== "assistant") continue;
		if (typeof m.content === "string") return m.content;
		if (Array.isArray(m.content)) {
			return m.content
				.filter((p) => p?.type === "text" && typeof p.text === "string")
				.map((p) => p.text)
				.join(" ");
		}
		return "";
	}
	return "";
};

export default function (pi: ExtensionAPI) {
	const pane = process.env.TMUX_PANE;
	if (!process.env.TMUX || !pane) return;

	let lastText = "";
	pi.on("agent_end", (e) => {
		lastText = lastAssistantText(e.messages ?? []);
	});

	pi.on("agent_settled", async () => {
		const [active, windowActive, attached, where] = (
			await tmux(["display-message", "-t", pane, "-p", "#{pane_active}\t#{window_active}\t#{session_attached}\t#{window_index}:#{window_name}"])
		).split("\t");
		if (!where || (active === "1" && windowActive === "1" && attached !== "0")) return;

		const text = lastText.replace(/[\x00-\x1f\x7f]+/g, " ").replace(/\s+/g, " ").trim();
		const body = text.length > MAX_BODY ? `${text.slice(0, MAX_BODY - 1)}…` : text || "ready";
		// '#' would be read as a tmux format
		await tmux(["display-message", `[${where}] pi: ${body.replaceAll("#", "##")}`]);
	});
}
