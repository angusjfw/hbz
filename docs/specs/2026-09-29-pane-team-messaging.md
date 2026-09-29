# pane-team messaging across harnesses

Let Claude Code and pi sessions in a pane-team message each other in
either direction. Each harness keeps its native messaging. `amx msg`
covers the cases native messaging can't reach, and pane-team says when
to use which.

## Framing

pane-team is part of amx: amx handles session mechanics, and pane-team
handles working alongside other agents in visible panes. Both build on
three things:

- tmux interaction (the `tmux-interaction` skill) for typing into and
  reading panes.
- Claude Code's native messaging (`SendMessage`, `ListAgents`).
- pi's session-control extension (agent-stuff `control.ts`): a
  per-session Unix socket at `~/.pi/session-control/<id>.sock`
  speaking newline-delimited JSON (`send`, `get_message`, ...).

A manager session is common but never required. Nothing in this spec
depends on a running manager or on the target having a registry entry.

Native messaging is the default and stays untouched: Claude → Claude
and pi → pi (with session-control) are the common cases and already
work. This design adds only what they can't reach, and keeps it out of
the way of agents that don't need it.

Codex has no native route. Typing into its pane is the general
fallback, so Codex targets are typed to like any other chat agent.

## Routes

| From → to | Route |
| --- | --- |
| Claude → Claude | `SendMessage` |
| pi → pi, target has a live socket | `send_to_session` |
| Claude or Codex → pi, target has a live socket | `amx msg`, over the socket |
| pi or Codex → Claude | `amx msg`, typed into the pane |
| any → pi without a live socket | `amx msg`, typed into the pane |
| any → Codex | `amx msg`, typed into the pane |
| any → generic command pane | refused |

Reading another agent's output is capture-pane via tmux-interaction.
For pi with a socket, `get_message` also works. There is no `amx read`.

"Live socket" is always probed at send time: connect to
`~/.pi/session-control/<session_id>.sock` using the resolved native
session ID. Recorded launch args are not trusted. A pi session can have
the extension enabled without `--session-control` in amx's records.

## `amx msg`

```
amx msg TARGET (TEXT | --file F) [--via auto|socket|pane] [--steer]
        [--wait SECS] [--force]
```

- `TARGET` is anything `resolve_target` accepts: an amx name or an
  explicit `%pane` ID. Dead, ambiguous or unresolved targets are
  refused with the resolver's error. Ambiguous names point to pane IDs.
- `--via auto` (default) picks the route from the table. `socket` and
  `pane` force one. A forced route that can't work is an error, not a
  silent switch.
- When the sender and target could use native messaging (Claude → Claude,
  or pi → pi with both sockets live, since pi has `send_to_session` only
  when it runs session-control itself), `auto` refuses and names the
  native tool. `--via pane` overrides this, e.g. when a permission-mode
  mismatch holds a native message for approval.
- `--steer` sends the socket message in steer mode (interrupts the
  current turn). The default is `follow_up`. It has no effect when typing.
- Result: one line on stdout, `delivered via=<socket|pane> target=<name>
  pane=<%id> verified=<yes|no>`, exit 0. A refusal is one line on
  stderr, `refused <reason>: <detail>`, exit 1. Reasons: `dead`,
  `ambiguous`, `unresolved`, `self`, `native`, `command`, `busy`,
  `needs-input`, `draft`, `box-unknown`, `state-unknown`, `no-socket`.
- `resolve_target`'s view gains `state`, the pane's agent-status state
  (`idle`, `working`, `done`, `needs_input`, `error`, `off`, or none).

### Sender header

Every message starts with a header built from `amx whoami` for
`$TMUX_PANE`:

```
[from <name> (<harness>, <%pane>); reply: amx msg <name> "..."]
```

`<name>` is the amx name, or the pane ID when the sender has no entry.
When pi sends to pi over the socket (`--via socket`), the body is
followed by the extension's `<sender_info>{"sessionId": ...}</sender_info>`
tag instead, so pi's own reply path works. Only the session ID goes in:
pi may look up `sessionName` as its own alias, which differs from the
amx name, and only a sender with its own live socket gets the tag. A
sender whose pane can't be resolved is named by its pane ID with harness
`unknown`. A sender outside tmux gets `[from an agent outside tmux; no
reply route]`.

### Socket delivery

Write one JSON line, `{"type":"send","message":...,"mode":...}`, then
read the newline-terminated response. `success: true` means `delivered`,
with `verified=yes`. A connect failure under `--via auto` falls through
to typing. Under `--via socket` it is refused as `no-socket`. A failure
after connecting is an error, never a fall-through, since the message
may already have landed.

### Typing into the pane

Escape can interrupt a running turn, and keys typed into a waiting
prompt can answer it. The sequence therefore refuses rather than
guesses:

1. **Busy check.** `working` is refused as `busy`, or waits with
   `--wait SECS`, polling until it's not working or the time runs out.
   No hook fires when the user interrupts a Claude turn, so `working`
   can go stale: `--force` overrides it, for an agent that has looked at
   the pane and seen it idle. No state (or `off`) is refused as
   `state-unknown` unless `--force` is given.
2. **Input box check.** Claude: `read-input-box.py`. pi: the editor is
   the text between the last two horizontal rules on screen, and pi
   shows no placeholder text. `empty` or `ghost` proceed. `draft` is
   refused, and `--force` does not override it. `no-box` (a dialog is
   up, or no editor on screen) is refused as `box-unknown`, also not
   overridable, and so is a pane in tmux copy mode, which eats Escape
   and Enter. Codex has no reader, so it is refused as `box-unknown`
   unless `--force` is given. `needs_input` is refused as `needs-input`,
   never forced, unless the box reads `empty` or `ghost`: cancelling a
   prompt fires no hook, and the box back on screen shows it's gone.
3. **Deliver.** Write the header and body to a temp file, load it into
   a tmux buffer and paste it with bracketed paste (`paste-buffer -p`).
   Bracketed paste lands intact in both vim INSERT and NORMAL mode. After
   a short pause, send Escape only if the pane shows an INSERT indicator
   (Claude `-- INSERT --`, pi's bottom rule ending in `INSERT`), then
   another pause and Enter. Without vim mode there's no indicator, and
   Enter submits.
4. **Verify.** Poll the box for a few seconds. `empty` or `ghost` means
   `verified=yes`. For Codex, or a box still holding text, report
   `verified=no`. Never report success on a hunch.

Checks and typing run under a lock per target pane, so two senders
take turns rather than merging pastes or typing into a turn the other
just started.

`--force` exists for an agent that knows nobody is typing in the target
pane. `amx reference` says so, and says never to use it on a pane the
user may be using.

## Skills

Agents that only need to send a message should not have to load a
skill. Both descriptions say when the skill isn't needed and give the
messaging answer directly.

**pane-team description** gains:

> Not needed just to message another session: use your harness's native
> tool (SendMessage, send_to_session) for the same CLI, or
> `amx msg <name> "..."` for a different one.

**tmux-interaction description** gains:

> Not for messaging another agent session: use native messaging
> (SendMessage, send_to_session), or `amx msg` across CLIs.

**pane-team body.** The Talk section keeps native messaging as its
first instruction. The "fall back to tmux send-keys" line becomes one
short paragraph: when the target runs a different CLI, or no native
tool reaches it, use `amx msg <name>`. An agent knows the target's
harness from its brief or spawn's `harness=` output, so the common
case needs no lookup. Refusal reasons and `--force` live in
`amx reference`, not the skill. Also add:

- The framing line: part of amx, built on tmux interaction, Claude
  messaging and pi session-control, manager optional.
- Mixed harnesses are normal. A worker may run a different CLI from its
  spawner. Spawn with `amx spawn --into` and the worker's own harness.
- A pane or window the user asked for is created in the current tmux
  session and never becomes a new session.
- Briefs say how the worker reports back, naming the route (`amx msg
  <name>` for a sender on another harness).

## amx docs

- `REFERENCE.md`: an `amx msg` section with the flags, result line,
  refusal reasons and what to do after each. `busy`: wait (`--wait`)
  or retry later. `draft`: ask the user. `box-unknown` /
  `state-unknown`: look at the pane, then decide on `--force`.
  `native`: use the named tool. `command`: use tmux-interaction
  deliberately. `needs-input`: tell the user the target is waiting on
  them.
- `HARNESSES.md`: a messaging line per harness (native tool, socket or
  none, typing supported). Codex: no native route, typing only.

## Testing

- Unit tests (style of `test_harnesses.py`): route choice from resolver
  output and socket liveness; each refusal reason; header for a
  registered sender, an unregistered sender and a sender outside tmux;
  forced routes don't fall through.
- `IsolatedTmuxCase` tests for typing: a stub prompt program records what
  it receives, covering busy refusal, `--wait`, paste and submit, and
  verification.
- A fake Unix socket server for socket delivery, including a connect
  failure falling through under `auto`.
- Manual live check, once, in throwaway panes: pi → Claude (typed),
  Claude → pi (socket), Claude → pi without session-control (typed),
  Claude → Codex (typed).
  These probes also fix the key sequences for each harness.

## Out of scope

- A receiver for Claude sessions (a channel server or the private peer
  socket). Typing is the Claude inbound route from other harnesses.
- A native Codex route.
- Waiting for a reply (`subscribe turn_end`), and `amx read`.
