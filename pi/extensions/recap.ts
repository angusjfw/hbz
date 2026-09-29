/**
 * /recap, after Claude Code's. Summarises the current branch with a one-off
 * call to the session's model and shows it in a panel; nothing enters the
 * session context. `/recap <focus>` steers what the recap concentrates on.
 */

import type { ExtensionAPI, ExtensionCommandContext } from "@earendil-works/pi-coding-agent";
import { DynamicBorder, getMarkdownTheme } from "@earendil-works/pi-coding-agent";
import { Container, Markdown, matchesKey, Text } from "@earendil-works/pi-tui";

const MAX_ARGS = 200;

type Block = { type?: string; text?: string; name?: string; arguments?: unknown };

const blocks = (content: unknown): Block[] =>
	typeof content === "string" ? [{ type: "text", text: content }] : Array.isArray(content) ? content : [];

const transcript = (entries: any[]): string => {
	const out: string[] = [];
	for (const e of entries) {
		if (e.type === "compaction" || e.type === "branch_summary") {
			out.push(`Summary of earlier work:\n${e.summary}`);
			continue;
		}
		const role = e.type === "message" ? e.message?.role : undefined;
		if (role !== "user" && role !== "assistant") continue;
		const lines: string[] = [];
		for (const b of blocks(e.message.content)) {
			if (b.type === "text" && b.text?.trim()) lines.push(`${role === "user" ? "User" : "Assistant"}: ${b.text.trim()}`);
			else if (b.type === "toolCall" && b.name) {
				const args = JSON.stringify(b.arguments ?? {});
				lines.push(`Tool ${b.name} ${args.length > MAX_ARGS ? `${args.slice(0, MAX_ARGS)}…` : args}`);
			}
		}
		if (lines.length) out.push(lines.join("\n"));
	}
	return out.join("\n\n");
};

const prompt = (conversation: string, focus: string) =>
	[
		"Write a short recap of this coding session for someone returning to it.",
		"Cover the goal, where things stand now, key decisions, and the next step.",
		"Be terse: a few bullets or short lines, no preamble.",
		focus ? `Focus on: ${focus}` : "",
		"",
		"<conversation>",
		conversation,
		"</conversation>",
	].join("\n");

const show = (recap: string, ctx: ExtensionCommandContext) =>
	ctx.ui.custom((_tui, theme, _kb, done) => {
		const box = new Container();
		const border = new DynamicBorder((s: string) => theme.fg("accent", s));
		box.addChild(border);
		box.addChild(new Text(theme.fg("accent", theme.bold("Recap")), 1, 0));
		box.addChild(new Markdown(recap, 1, 1, getMarkdownTheme()));
		box.addChild(new Text(theme.fg("dim", "Enter or Esc to close"), 1, 0));
		box.addChild(border);
		return {
			render: (width: number) => box.render(width),
			invalidate: () => box.invalidate(),
			handleInput: (data: string) => {
				if (matchesKey(data, "enter") || matchesKey(data, "escape")) done(undefined);
			},
		};
	});

export default function (pi: ExtensionAPI) {
	pi.registerCommand("recap", {
		description: "Recap the session so far (optional focus)",
		handler: async (args, ctx) => {
			if (ctx.mode !== "tui") return;
			const conversation = transcript(ctx.sessionManager.getBranch());
			if (!conversation.trim()) return ctx.ui.notify("Nothing to recap yet", "warning");
			const model = ctx.model;
			if (!model) return ctx.ui.notify("No model selected", "warning");

			ctx.ui.setStatus("recap", "recapping…");
			try {
				const res = await ctx.modelRegistry.complete(
					model,
					{ messages: [{ role: "user", content: [{ type: "text", text: prompt(conversation, args.trim()) }], timestamp: Date.now() }] },
					model.reasoning ? { reasoning: "low" } : {},
				);
				if (res.stopReason === "error") throw new Error(res.errorMessage ?? "request failed");
				const recap = res.content
					.filter((c): c is { type: "text"; text: string } => c.type === "text")
					.map((c) => c.text)
					.join("\n")
					.trim();
				if (!recap) return ctx.ui.notify("Recap came back empty", "warning");
				await show(recap, ctx);
			} catch (err) {
				ctx.ui.notify(`Recap failed: ${err instanceof Error ? err.message : String(err)}`, "error");
			} finally {
				ctx.ui.setStatus("recap", undefined);
			}
		},
	});
}
