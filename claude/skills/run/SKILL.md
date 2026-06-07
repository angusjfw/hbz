---
name: run
description: Launch and drive this project's app to see a change working. Use when asked to run, start, or screenshot the app, or to confirm a change works in the real app (not just tests). First looks for a project skill that already covers launching the app; otherwise reads the repo's own launch docs (README, Taskfile, Makefile, Procfile, docker-compose, bin/dev); only falls back to built-in patterns per project type if none of those describe how to launch.
---

**Running means launching the actual app and interacting with it** —
not the test suite, not an `import` of an internal function and a
`console.log`. The app as a user (human or programmatic) would meet
it: the CLI at its command, the server at its socket, the GUI at its
window.

## First: does a project skill already cover this?

A project skill that launches this app is the repo's verified path —
its author already cold-started from a Linux container and committed
what worked: the exact `apt-get` line, the env vars, the patches, the
driver. Use it instead of rediscovering.

```bash
d=$PWD; while :; do
  grep -Hm1 '^description:' "$d"/.claude/skills/*/SKILL.md 2>/dev/null
  [ -e "$d/.git" ] || [ "$d" = / ] && break
  d=$(dirname "$d")
done
```

- **One describes launching/driving this app** → read that SKILL.md
  and follow it verbatim. Don't paraphrase; don't skip the patches.
- **Mega-repo, several plausible, no clear match** → ask the user
  which unit to run.
- **Stale** (fails on mechanics unrelated to your task) → tell the
  user; offer to refresh it via `/run-skill-generator`.
- **Nothing about running** → go to the next step.

## Next: does the repo document how to launch?

Most repos already have a verified launch path — the team uses it
daily. Read it before reaching for a generic pattern. Check in this
order:

1. **README** — look for a section headed "Run", "Running", "Getting
   started", "Quickstart", "Development", "Run the app", "Run locally",
   or similar. `grep -niE '^#+ ?(run|running|getting started|quickstart|develop|local|setup)' README.md` is a fast scan. Read the matched section in full.
2. **Taskfile.yml** — `go-task` runner. Look for tasks named `run`,
   `dev`, `up`, `start`, `serve`. `task --list` if installed.
3. **Makefile** — look for `run`, `dev`, `up`, `start` targets.
4. **Procfile / Procfile.dev** — process orchestration (Foreman / Overmind / Honcho).
5. **docker-compose.yml / compose.yaml** — `docker compose up` may be the documented path.
6. **bin/dev**, **script/server**, **scripts/dev** — repo-local launch shims.
7. **package.json `scripts`** — only when the README explicitly points at a script (e.g. "run `npm run dev`"). Don't pick a script just because it exists; that's the same shortcut as the pattern matrix.

**If any of these describe a launch path, follow it.** Don't substitute
a simpler-looking command from `package.json` because it "looks like the
same thing." Multi-process orchestration (mock servers, queue workers,
DB migrations, dev shells) frequently hides behind a `task run` or
`make dev` and is invisible from `package.json` alone.

Cross-check against what's actually running on the machine before
starting external services from scratch — a Postgres or RabbitMQ may
already be up under a different name (Docker container, native
service, devenv-managed process). Skip launch steps for things
already running; start the rest.

If the docs are wrong (commands fail, paths drifted), tell the user
what diverged and propose the fix — don't silently work around it.

## Otherwise: match the shape, use the pattern

Only when the repo offers no launch documentation. Pick the row
closest to your project. Each example walks through launch + first
interaction; ignore any trailing "write the skill" section — you're
using the recipe, not authoring one.

| Project type | Handle | Example |
|---|---|---|
| CLI tool | direct invocation, exit code, stdin/stdout | [examples/cli.md](examples/cli.md) |
| Web server / API | background launch + `curl` smoke | [examples/server.md](examples/server.md) |
| TUI / interactive terminal | tmux `send-keys` / `capture-pane` | [examples/tui.md](examples/tui.md) |
| Electron / desktop GUI | Playwright `_electron` REPL under xvfb | [examples/electron.md](examples/electron.md) |
| Browser-driven | dev server + `chromium-cli` script | [examples/playwright.md](examples/playwright.md) |
| Library / SDK | import-and-call smoke script at the package boundary | [examples/library.md](examples/library.md) |

If nothing fits, start from the closest match and adapt. For a web
app, [examples/playwright.md](examples/playwright.md) — drive it with
`chromium-cli`, no custom driver needed. For a desktop app,
[examples/electron.md](examples/electron.md) — it has the `_electron`
REPL driver skeleton and the tmux wrapping.

## Drive it, don't just launch it

Launching with no interaction proves the entrypoint resolves. That's
not running the app — it's typechecking with extra steps. Drive it to
a point where a user would see something:

- CLI → type a representative command, check the exit code and output.
- Server → hit the route the diff touches with `curl`, read the body.
- TUI → `send-keys` a navigation, `capture-pane` the result.
- GUI → click the button, screenshot the window. **Look at the
  screenshot.** A blank frame is a failure to launch.

If you ended up using the pattern-matrix fallback (no project skill,
no repo launch docs) and it didn't work out of the box — you had to
install packages, set env vars, patch config, or write a driver —
recommend `/run-skill-generator` in your report so that work gets
captured as a project skill. If you followed a documented path from
the repo, no skill is needed; the docs already are the verified path.
