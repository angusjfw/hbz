/**
 * Two-line footer. The first line follows the Claude Code status line
 * (claude/hooks/statusline.sh): cwd · branch · session name · model ·
 * thinking. The second keeps everything pi's own footer shows: context use,
 * token and cache stats, cost, and extension statuses.
 */

import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

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

const fmt = (n: number) =>
	n < 1000 ? `${n}` : n < 10_000 ? `${(n / 1000).toFixed(1)}k` : n < 1e6 ? `${Math.round(n / 1000)}k` : `${(n / 1e6).toFixed(1)}M`;

type Usage = { input: number; output: number; cacheRead: number; cacheWrite: number; cost: { total: number } };

const autoCompact = () => {
	try {
		const settings = JSON.parse(readFileSync(join(homedir(), ".pi/agent/settings.json"), "utf8"));
		return settings.compaction?.enabled !== false;
	} catch {
		return true;
	}
};

export default function (pi: ExtensionAPI) {
	pi.on("session_start", (_e, ctx) => {
		if (ctx.mode !== "tui") return;
		const auto = autoCompact() ? " (auto)" : "";

		ctx.ui.setFooter((tui, _theme, footerData) => {
			const unsub = footerData.onBranchChange(() => tui.requestRender());
			return {
				dispose: unsub,
				invalidate() {},
				render(width: number): string[] {
					const home = process.env.HOME ?? "";
					const cwd = home && ctx.cwd.startsWith(home) ? `~${ctx.cwd.slice(home.length)}` : ctx.cwd;
					const top = [`${D}${cwd}${RST}`];
					const branch = footerData.getGitBranch();
					if (branch) top.push(`${G}${branch}${RST}`);
					const name = pi.getSessionName();
					if (name) top.push(`${Y}${name}${RST}`);
					if (ctx.model) top.push(`${D}${ctx.model.provider}/${RST}${B}${C}${ctx.model.id}${RST}`);
					if (ctx.model?.reasoning && ctx.thinkingLevel) top.push(`${M}${ctx.thinkingLevel}${RST}`);

					// same accounting as pi's footer: every entry that carries usage
					const t = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, cost: 0 };
					let hitRate: number | undefined;
					const add = (u: Usage | undefined) => {
						if (!u) return;
						t.input += u.input;
						t.output += u.output;
						t.cacheRead += u.cacheRead;
						t.cacheWrite += u.cacheWrite;
						t.cost += u.cost?.total ?? 0;
					};
					for (const e of ctx.sessionManager.getEntries() as any[]) {
						if (e.type === "usage") add(e.usage);
						else if (e.type === "message" && e.message.role === "assistant") {
							const u = e.message.usage as Usage;
							add(u);
							const prompt = u.input + u.cacheRead + u.cacheWrite;
							hitRate = prompt > 0 ? (u.cacheRead / prompt) * 100 : undefined;
						} else if (e.type === "message" && e.message.role === "toolResult") add(e.message.usage);
						else if (e.type === "branch_summary" || e.type === "compaction") add(e.usage);
					}

					const usage = ctx.getContextUsage();
					const window = usage?.contextWindow ?? ctx.model?.contextWindow ?? 0;
					const pct = usage?.percent;
					const ctxColour = (pct ?? 0) >= 90 ? R : (pct ?? 0) >= 70 ? Y : G;
					const bottom = [`${ctxColour}${pct == null ? "?" : pct.toFixed(1)}%${RST}${D}/${fmt(window)}${auto}${RST}`];

					const stats: string[] = [];
					if (t.input) stats.push(`↑${fmt(t.input)}`);
					if (t.output) stats.push(`↓${fmt(t.output)}`);
					if (t.cacheRead) stats.push(`R${fmt(t.cacheRead)}`);
					if (t.cacheWrite) stats.push(`W${fmt(t.cacheWrite)}`);
					if ((t.cacheRead || t.cacheWrite) && hitRate !== undefined) stats.push(`CH${hitRate.toFixed(1)}%`);
					if (stats.length) bottom.push(`${D}${stats.join(" ")}${RST}`);
					if (t.cost) bottom.push(`$${t.cost.toFixed(3)}`);

					for (const text of footerData.getExtensionStatuses().values()) bottom.push(text);

					return [truncateToWidth(top.join(SEP), width), truncateToWidth(bottom.join(SEP), width)];
				},
			};
		});
	});
}
