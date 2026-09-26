/**
 * One-line footer in the same layout and colours as the Claude Code status line
 * (claude/hooks/statusline.sh): cwd · branch · ctx% · model · thinking, then
 * session cost, session name and any extension statuses worth showing.
 */

import type { AssistantMessage } from "@earendil-works/pi-ai";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { truncateToWidth } from "@earendil-works/pi-tui";

const D = "\x1b[2m";
const B = "\x1b[1m";
const G = "\x1b[32m";
const Y = "\x1b[33m";
const R = "\x1b[31m";
const C = "\x1b[36m";
const M = "\x1b[35m";
const RST = "\x1b[0m";
const SEP = `${D} · ${RST}`;

export default function (pi: ExtensionAPI) {
	pi.on("session_start", (_e, ctx) => {
		if (ctx.mode !== "tui") return;

		ctx.ui.setFooter((tui, _theme, footerData) => {
			const unsub = footerData.onBranchChange(() => tui.requestRender());
			return {
				dispose: unsub,
				invalidate() {},
				render(width: number): string[] {
					const home = process.env.HOME ?? "";
					const cwd = home && ctx.cwd.startsWith(home) ? `~${ctx.cwd.slice(home.length)}` : ctx.cwd;
					const parts = [`${D}${cwd}${RST}`];

					const branch = footerData.getGitBranch();
					if (branch) parts.push(`${G}${branch}${RST}`);

					const pct = Math.round(ctx.getContextUsage()?.percent ?? 0);
					const ctxColour = pct >= 90 ? R : pct >= 70 ? Y : G;
					parts.push(`${ctxColour}${pct}%${RST}`);

					if (ctx.model) parts.push(`${B}${C}${ctx.model.id}${RST}`);
					if (ctx.model?.reasoning && ctx.thinkingLevel) parts.push(`${M}${ctx.thinkingLevel}${RST}`);

					let cost = 0;
					for (const e of ctx.sessionManager.getBranch()) {
						if (e.type === "message" && e.message.role === "assistant") {
							cost += (e.message as AssistantMessage).usage.cost.total;
						}
					}
					if (cost > 0) parts.push(`${D}$${cost.toFixed(3)}${RST}`);

					const name = pi.getSessionName();
					if (name) parts.push(`${Y}${name}${RST}`);

					for (const [key, text] of footerData.getExtensionStatuses()) {
						// the session name already identifies a controllable session
						if (key === "session-control") continue;
						// auto mode's status only matters while it's on
						if (key === "pi-automode" && text.includes("○")) continue;
						parts.push(text);
					}

					return [truncateToWidth(parts.join(SEP), width)];
				},
			};
		});
	});
}
