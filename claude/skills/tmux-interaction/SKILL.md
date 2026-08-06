---
name: tmux-interaction
description: Use when sending input to or reading output from a tmux pane other than the one in focus: running a command, dev server, REPL, or long-running process in a pane and waiting for it to print a prompt, port, or result; reading or scrolling pane output; or driving a shell, vim/neovim, or another agent via send-keys and capture-pane. Also when input appears to land but doesn't submit, when keystrokes get dropped, or when another agent's prompt box looks like it holds something the user typed.
---

# tmux-interaction

Patterns for Claude to read from and write to tmux panes. Targets
include other Claude conversations, neovim, REPLs, or any
interactive program; the patterns are the same. Borrows the polling
helper from `mitsuhiko/agent-stuff`; see `NOTICE`.

## Sending input

Literal sends to avoid shell splitting:

```bash
tmux send-keys -t <target> -l -- "$cmd"
tmux send-keys -t <target> Enter
```

`<target>` is `<session>:<window>.<pane>`, defaulting to `:0.0` if
omitted. Control keys: `C-c`, `C-d`, `Escape`.

Send the payload and the Enter as separate calls. Mixing literal
text and key tokens in the same call has edge cases (especially with
quoting and modal editors); the separate-call form is the robust
default. There are exceptions where mixed sends are fine (a known
short nvim command, for instance), but lean separate.

## Vim-mode targets

If the target is vim or runs through a vim-mode editor — neovim,
or a Claude conversation with `editorMode: vim` — its modal nature
shapes how input lands. **Start with `Escape`** to put it in NORMAL
mode before assuming anything about where keystrokes go:

```bash
tmux send-keys -t <target> Escape
sleep 0.3
# ...subsequent ops
```

This is cheap insurance and applies to any vim-mode interaction,
not just submitting prompts.

For Claude with `editorMode: vim`, plain `Enter` in INSERT just adds
a newline. To submit you need `Escape` -> brief delay -> `Enter`:

```bash
tmux send-keys -t <target> -l -- "$message"
tmux send-keys -t <target> Escape
sleep 0.3
tmux send-keys -t <target> Enter
```

A direct `send-keys ... Enter` on a vim-mode prompt looks like it
lands but never submits. If a send seems to land but nothing
happens, suspect this.

## Lost leading character

When chaining `Escape` and then more text into a vim-mode target,
the first character of the next text can be eaten — input briefly
returns to NORMAL and consumes the keystroke as a vim command before
INSERT resumes.

Workaround: prepend an explicit `i` so the first character is the
INSERT-mode trigger, not real content:

```bash
tmux send-keys -t <target> Escape
sleep 0.3
tmux send-keys -t <target> -l -- "i$message"
```

Or write the message to a file and have the receiver read it,
sidestepping send-keys quoting and mode entirely:

```bash
f=$(mktemp -t claude-msg.XXXXXX)
printf '%s\n' "$message" > "$f"
tmux send-keys -t <target> -l -- "cat $f"
tmux send-keys -t <target> Enter
```

The file route is more robust for multi-line or quote-heavy
messages.

## Reading pane output

Capture the pane and inspect its recent output:

```bash
tmux capture-pane -p -J -t <target> -S -<n>
```

`-J` joins wrapped lines (don't skip it). `-S -<n>` reads the last
N lines from history; tune to taste.

Use this after every send to verify the input landed where expected.
Don't assume the send worked just because the command exited
cleanly — modal editors and lost characters can swallow input
silently.

Two flags shape what the output means. `-p` strips colour, so anything
the TUI distinguishes only by colour arrives flattened — see reading an
agent's input box below. And `-S -<n>` starts `<n>` lines back in
*history* and still runs to the end of the screen; for "the bottom N
rows of what's on screen", capture the visible region explicitly:

```bash
h=$(tmux display-message -p -t <target> '#{pane_height}')
tmux capture-pane -p -t <target> -S $((h - N)) -E $((h - 1))
```

## Reading an agent's input box

The box at the bottom of a Claude Code pane is not a message log. While
it's empty, Claude fills it with a **dim placeholder guessing what the
user might type next**. That text is Claude's, not the user's — but a
plain `capture-pane -p` strips the dimness, leaving it byte-identical to
something the user really typed. Believing it is how an agent ends up
acting on instructions nobody gave, or sitting on its hands because it
thinks the user is mid-sentence.

Classify the box rather than reading it:

```bash
scripts/read-input-box.py <pane-target>
```

It prints the state, plus a tab and the box text when there is any:

- `no-box` — nothing to read: a dialog is up (permission prompt,
  selection list), or the pane isn't running Claude.
- `empty` — box present, nothing in it.
- `ghost` — the box holds only the dim placeholder. Not user text;
  ignore it, and don't quote it back at anyone.
- `draft` — real, unsent user text. Don't clobber it and don't send keys
  into that pane; the user is mid-typing.

Neither state is a message *to* you. Only submitted messages count —
those sit in the transcript above the box, on a highlighted background.
Anything still in the box is at most "the user is typing".

By hand, if the script isn't available: capture with `-e` to keep the
SGR escapes and look at what follows the `❯` marker. Placeholder text is
wrapped in `ESC[2m` … `ESC[0m`; a real draft carries no dim run.

Details that bite:

- The marker is `❯` followed by U+00A0, not a space. Selection-list rows
  (`❯ 1. Yes`) reuse the glyph with a normal space — that NBSP is what
  tells the input box apart from a dialog.
- While Claude is working the marker itself renders dim. Strip the
  marker before testing the content, or every busy pane reads as a
  placeholder.
- Cursor column corroborates, it doesn't decide. `cursor_x > 2` proves a
  real draft, but a placeholder and a draft whose author moved the cursor
  back to the start both sit at `cursor_x == 2`.
- Tab turns the placeholder into a genuine draft — same text, no longer
  dim. "It was a placeholder a second ago" doesn't make it fake now.
- `-J` joins wrapped lines and breaks the row-to-`cursor_y` mapping.
  Leave it off when screen position matters.
- The box's top border can carry a branch or PR badge, and its colour
  varies, so "a line of `─`" doesn't locate the box on its own.

## Polling for output

`scripts/wait-for-text.sh` polls a pane for a regex (or fixed string)
with a timeout. Use this instead of blind `sleep` when waiting for a
response or prompt:

```bash
scripts/wait-for-text.sh -t <target> -p '^>' -T 30 -i 0.5
```

- `-t` pane target, required
- `-p` regex pattern, required
- `-F` fixed string instead of regex
- `-T` timeout seconds (default 15)
- `-i` poll interval seconds (default 0.5)
- `-l` history lines to inspect (default 1000)

Exits 0 on first match, 1 on timeout. On timeout, prints the last
captured pane content to stderr.
