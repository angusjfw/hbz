/**
 * Prompt stash, after Claude Code's ctrl+s. With a draft in the editor, ctrl+s
 * sets it aside and clears the editor; the draft comes back once the next
 * prompt is submitted. ctrl+s on an empty editor brings it back straight away.
 * One slot, in memory only.
 */

import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
	let stash: string | undefined;

	const set = (ctx: ExtensionContext, text: string | undefined) => {
		stash = text;
		ctx.ui.setStatus("stash", text === undefined ? undefined : "stash");
	};

	pi.registerShortcut("ctrl+s", {
		description: "Stash the editor draft, or restore it",
		handler: (ctx) => {
			const draft = ctx.ui.getEditorText();
			if (draft.trim()) {
				set(ctx, draft);
				ctx.ui.setEditorText("");
			} else if (stash !== undefined) {
				ctx.ui.setEditorText(stash);
				set(ctx, undefined);
			}
		},
	});

	pi.on("input", (event, ctx) => {
		if (stash === undefined || event.source !== "interactive") return;
		const text = stash;
		set(ctx, undefined);
		// the submit path clears the editor around this event; restore after it
		setTimeout(() => ctx.ui.setEditorText(text), 0);
	});
}
