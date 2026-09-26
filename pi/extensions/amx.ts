/**
 * amx in pi: session hooks for every interactive session, and the registry
 * watch a manager starts.
 *
 * Hooks: `amx hook --harness pi` gets the same JSON Claude Code's
 * SessionStart/SessionEnd hooks send, so amx logs the session and keeps the
 * registry in step. Headless children (subagents) inherit TMUX_PANE, so only
 * the interactive session reports.
 *
 * Watch: the manager calls the `amx_watch` tool to start it; until then it
 * does nothing. It watches the registry's directory (a rewrite replaces the
 * file), skips writes made from this pane (amx records the writer's
 * $TMUX_PANE in .last-writer), and queues one follow-up message per change;
 * pi queues follow-ups sent mid-turn itself.
 */

import { spawn } from "node:child_process";
import * as fs from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

import { StringEnum } from "@earendil-works/pi-ai";
import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";

const AMX = join(homedir(), ".local/bin/amx");
const STATE = process.env.AMX_STATE_DIR || join(homedir(), ".local/state/amx");
const REGISTRY = "sessions.md";
const DEBOUNCE_MS = 500;

export default function (pi: ExtensionAPI) {
	const hook = (event: string, ctx: ExtensionContext, extra: Record<string, string>) => {
		if (ctx.mode !== "tui" || !fs.existsSync(AMX)) return;
		const payload = JSON.stringify({
			hook_event_name: event,
			session_id: ctx.sessionManager.getSessionId(),
			cwd: ctx.cwd,
			...extra,
		});
		const child = spawn(AMX, ["hook", "--harness", "pi"], { stdio: ["pipe", "ignore", "ignore"], detached: true });
		child.on("error", () => {});
		child.stdin.end(payload);
		child.unref();
	};

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

	// only ever a tmux pane id: the file's contents never reach the prompt otherwise
	const lastWriter = () => {
		try {
			const path = join(STATE, ".last-writer");
			if (!fs.lstatSync(path).isFile()) return "";
			const v = fs.readFileSync(path, "utf8").slice(0, 32).trim();
			return /^%\d+$/.test(v) ? v : "";
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
				customType: "amx-watch",
				content: `amx registry changed (writer ${writer || "unknown"}). Run \`amx ls -a\` and \`amx reconcile\` and act on what changed.`,
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
		name: "amx_watch",
		label: "amx watch",
		description:
			"Start or stop watching the amx registry. While started, each change made by another pane arrives as a message asking you to reconcile. Only for an amx manager session.",
		parameters: Type.Object({
			action: StringEnum(["start", "stop"] as const, { description: "start or stop the watch" }),
		}),
		async execute(_id, params) {
			const text = params.action === "start" ? start() : (stop(), "stopped");
			return { content: [{ type: "text", text }], details: undefined };
		},
	});

	pi.on("session_start", (e, ctx) => hook("SessionStart", ctx, { source: e.reason }));
	pi.on("session_shutdown", (e, ctx) => {
		stop();
		// reload/new/resume/fork start a fresh session in the same pane; its session_start supersedes this one
		if (e.reason === "quit") hook("SessionEnd", ctx, { reason: "quit" });
	});
}
