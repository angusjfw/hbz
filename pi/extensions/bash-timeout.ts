/**
 * Give agent bash calls a default timeout. pi's bash tool has none, so one bad
 * command (on WSL, a `find /` that walks the Windows drive) can stall a session
 * for as long as it runs. The model can still pass its own timeout for long jobs.
 */

import { type ExtensionAPI, isToolCallEventType } from "@earendil-works/pi-coding-agent";

const DEFAULT_TIMEOUT_SECONDS = 300;

export default function (pi: ExtensionAPI) {
	pi.on("tool_call", (event) => {
		if (isToolCallEventType("bash", event) && event.input.timeout === undefined) {
			event.input.timeout = DEFAULT_TIMEOUT_SECONDS;
		}
	});
}
