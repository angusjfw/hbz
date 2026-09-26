/**
 * Registry watch for a claude-manager running in pi: the counterpart of Claude
 * Code's Monitor on `cm watch`. The manager calls the `cm_watch` tool to start
 * it; until then it does nothing, so ordinary pi sessions are unaffected.
 *
 * Watches the registry's directory (a rewrite replaces the file), skips writes
 * made from this pane (cm records the writer's $TMUX_PANE in .last-writer),
 * and queues one follow-up message per change. pi queues follow-ups sent
 * mid-turn itself.
 */

import * as fs from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

import { StringEnum } from "@earendil-works/pi-ai";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";

const STATE = process.env.CM_STATE_DIR || join(homedir(), ".local/state/claude-manager");
const REGISTRY = "sessions.md";
const DEBOUNCE_MS = 500;

export default function (pi: ExtensionAPI) {
	let watcher: fs.FSWatcher | undefined;
	let timer: NodeJS.Timeout | undefined;
	let lastMtime = 0;

	const mtime = () => {
		try {
			return fs.statSync(join(STATE, REGISTRY)).mtimeMs;
		} catch {
			return 0;
		}
	};

	const lastWriter = () => {
		try {
			return fs.readFileSync(join(STATE, ".last-writer"), "utf8").trim();
		} catch {
			return "";
		}
	};

	const onChange = () => {
		const m = mtime();
		if (m === lastMtime) return;
		lastMtime = m;
		const writer = lastWriter();
		if (writer && writer === process.env.TMUX_PANE) return;
		pi.sendMessage(
			{
				customType: "cm-watch",
				content: `claude-manager registry changed (writer ${writer || "unknown"}). Run \`cm reg ls\` and \`cm reconcile\` and act on what changed.`,
				display: true,
			},
			{ triggerTurn: true, deliverAs: "followUp" },
		);
	};

	const stop = () => {
		clearTimeout(timer);
		watcher?.close();
		watcher = undefined;
	};

	const start = () => {
		if (watcher) return "already watching";
		fs.mkdirSync(STATE, { recursive: true });
		lastMtime = mtime();
		watcher = fs.watch(STATE, (_event, file) => {
			if (file !== REGISTRY) return;
			clearTimeout(timer);
			timer = setTimeout(onChange, DEBOUNCE_MS);
		});
		return `watching ${join(STATE, REGISTRY)}`;
	};

	pi.registerTool({
		name: "cm_watch",
		label: "cm watch",
		description:
			"Start or stop watching the claude-manager registry. While started, each change made by another pane arrives as a message asking you to reconcile. Only for a claude-manager session.",
		parameters: Type.Object({
			action: StringEnum(["start", "stop"] as const, { description: "start or stop the watch" }),
		}),
		async execute(_id, params) {
			const text = params.action === "start" ? start() : (stop(), "stopped");
			return { content: [{ type: "text", text }], details: undefined };
		},
	});

	pi.on("session_shutdown", () => stop());
}
