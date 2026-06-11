# Apple containers for dev sessions: assessment (deferred)

Proposal: use Apple's container stack (apple/container,
apple/containerization) for isolated, freeze/resumable per-task dev
sessions — the local equivalent of Firecracker/sprites.dev snapshot
workflows. Motivations: parallel dev servers without port conflicts;
parking a task's environment for days and bringing it back.

Decision: **deferred**. The load-bearing capability doesn't exist yet.

## Findings (June 2026, container 1.0.0)

- **No memory freeze/resume.** Lifecycle verbs are
  create/run/start/stop/kill/delete only. The Containerization library
  has an internal VM `pause()` but never uses the Virtualization
  framework's `saveMachineStateTo`/`restoreMachineStateFrom` (memory
  snapshot to disk); no feature request exists. Firecracker-style
  suspend of a Linux environment on macOS is shipped by no tool —
  Tart suspends macOS guests only; Lima's Linux-guest attempt
  (lima-vm/lima PR #2900) is unmerged. sprites.dev (Fly.io) delivers
  exactly the wanted experience (300ms checkpoints, <1s restore) but
  is Firecracker/KVM, cloud-only.
- **Stop/start is the available shape.** Per-container filesystem
  persists across stop/start and host reboots; a stopped container
  boots in ~1s; processes do not survive. `container machine` (1.0.0)
  targets persistent dev environments with home-dir auto-mount.
- **Port isolation is solved and easy.** One lightweight VM per
  container, each with its own IP (`http://192.168.64.x:3000`,
  `--publish` for localhost, `<name>.test` DNS). N dev servers on
  :3000 with zero conflict. macOS 26 required for the full story.
- **Costs:** a Linux image of the full toolchain must be built and
  maintained; virtiofs file-sharing has unreliable inotify (polling
  watchers for dev servers); ~1–2 GB RAM per running container that
  ratchets (imperfect ballooning); no Docker socket/compose; breaking
  changes reserved until 2.0.

## Why deferred

Without memory freeze, the proposal reduces to per-task Linux VMs.
Parking gains little (worktree and node_modules already persist on the
host; amx resume replays processes either way). Port isolation is the
one clear win and is buyable far cheaper (per-worktree port
parametrization). The Linux-image and virtiofs costs don't clear that
bar today.

## Revisit triggers

- Memory snapshot/restore ships anywhere usable: apple/containerization
  adopts VZ save/restore, a checkpoint verb appears in
  apple/container, or lima-vm/lima PR #2900 merges.
- A sprites-like local product appears for macOS.
- Parallel dev-server pain grows enough to justify stop/start alone —
  then the shape is: one `container machine` per task, worktree
  mounted, amx resume_state replaying in-guest processes.

Related: agent sandboxes on this stack exist (CodeRunner,
SandboxedClaudeCode) if hard agent isolation becomes a separate want.
