/**
 * Feeds pi session state to `agent-status` (keyboard LEDs, session switcher),
 * translating pi events into the Claude Code hook events it already understands.
 * No-op outside tmux or when agent-status isn't installed.
 */

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";

const BIN = join(homedir(), ".local/bin/agent-status");
// tool calls are the "still working" heartbeat; one per burst is plenty
const HEARTBEAT_MS = 5000;

export default function (pi: ExtensionAPI) {
	if (!process.env.TMUX || !existsSync(BIN)) return;

	let lastHeartbeat = 0;

	const report = (event: string, ctx: ExtensionContext) => {
		const payload = JSON.stringify({
			hook_event_name: event,
			session_id: ctx.sessionManager.getSessionId(),
			cwd: ctx.cwd,
		});
		const child = spawn(BIN, ["event"], { stdio: ["pipe", "ignore", "ignore"], detached: true });
		child.on("error", () => {});
		child.stdin.end(payload);
		child.unref();
	};

	pi.on("session_start", (_e, ctx) => report("SessionStart", ctx));
	pi.on("agent_start", (_e, ctx) => report("UserPromptSubmit", ctx));
	pi.on("tool_execution_end", (_e, ctx) => {
		const now = Date.now();
		if (now - lastHeartbeat < HEARTBEAT_MS) return;
		lastHeartbeat = now;
		report("PostToolUse", ctx);
	});
	pi.on("agent_settled", (_e, ctx) => report("Stop", ctx));
	// a blocking prompt mid-run needs the user; one opened while idle, they're already at
	pi.on("ui_prompt_start", (_e, ctx) => {
		if (!ctx.isIdle()) report("Notification", ctx);
	});
	pi.on("ui_prompt_end", (_e, ctx) => {
		if (!ctx.isIdle()) report("PostToolUse", ctx);
	});
	// reload/new/resume/fork start a fresh session in the same pane, which supersedes this one
	pi.on("session_shutdown", (e, ctx) => {
		if (e.reason === "quit") report("SessionEnd", ctx);
	});
}
