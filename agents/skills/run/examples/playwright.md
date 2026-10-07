# Example: Browser-driven web app

You have a dev server that serves HTML to a browser. "Run the app" means
launching the dev server, driving a headless browser against it, and
producing a screenshot that proves the page rendered.

Don't write a browser driver. Use the `playwright` MCP tools (headless,
isolated, Playwright's own browser). Use `chrome-devtools` only when the
app needs the user's logged-in session; it attaches to their Chrome and
asks them to approve.

## Dev server

Find the dev command (`package.json` `scripts.dev`, `Makefile`,
README), start it in the background, and wait for it to actually serve:

```bash
npm run dev &   # or yarn dev, pnpm dev, make serve, ./dev.sh
echo $! > /tmp/dev.pid
timeout 30 bash -c 'until curl -sf http://localhost:3000 >/dev/null; do sleep 1; done'
```

Don't `sleep 5` — poll the port. Stop with
`kill $(cat /tmp/dev.pid)` (or `pkill -f 'npm run dev'`) before
relaunching, or the next run hits `EADDRINUSE`.

## Drive

The loop: `browser_navigate` to the dev URL → `browser_wait_for` the
text you need → act (`browser_click` / `browser_type` /
`browser_fill_form` / `browser_press_key`, using refs from
`browser_snapshot`) → `browser_take_screenshot` →
`browser_console_messages` to check nothing threw. `browser_resize`
for responsive checks.

The browser is isolated, so each session starts logged out, and it
closes after 10 minutes idle.

## What to put in the skill

The project-specific bits only. The MCP tools handle the mechanics.

- **Dev command + port + stop.** The exact start line, any env vars it
  needs, and the `kill`/`pkill` to stop it.
- **Auth.** Whatever gets a logged-in session — a login form sequence,
  or a helper script that does the API dance and sets the cookie via
  `browser_evaluate`.
- **One representative interaction.** Not the whole app — one path that
  proves it's running, ending in a screenshot.
- **App-specific gotchas.** Only the ones you actually hit.

## Gotchas that recur

- **React controlled inputs.** Setting `el.value` in
  `browser_evaluate` doesn't fire React's onChange. Use `browser_type`
  / `browser_fill_form` — they go through Playwright's input pipeline.
- **Websockets / long-poll.** Network-idle never settles.
  `browser_wait_for` the element you actually need.
- **Slow first paint.** Vite/Next compile routes on demand; the first
  navigation can take 10s+. Wait for text, not a fixed delay.
- **Element screenshots.** `browser_take_screenshot` with an element ref
  crops to one component — use it when the diff is in one place.
- **Check console errors before declaring success.** A page can render
  its shell while every data fetch 500s.
