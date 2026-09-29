"""Agent adapters and generic commands share tmux lifecycle, not resume semantics."""

import contextlib
import io
import json
import os
import shlex
import sqlite3
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from test_amx import load_cm
import test_startup

SID = "11111111-2222-3333-4444-555555555555"
OTHER = "11111111-2222-3333-4444-555556666666"
PREFIX = SID[:29] + "..."


class HarnessTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.cm = load_cm(self.root / "state")
        self.home = self.root / "codex"
        self.home.mkdir()
        env = patch.dict(os.environ, {"CODEX_HOME": str(self.home), "CODEX_SQLITE_HOME": str(self.home)})
        env.start()
        self.addCleanup(env.stop)

    def index(self, rows):
        with contextlib.closing(sqlite3.connect(self.home / "state_5.sqlite")) as db:
            db.execute("CREATE TABLE threads (id TEXT, cwd TEXT)")
            db.executemany("INSERT INTO threads VALUES (?, ?)", rows)
            db.commit()

    def test_codex_classification(self):
        for args in ("codex", "codex --model example", "/opt/bin/codex resume " + SID,
                     "node /opt/node_modules/@openai/codex/bin/codex.js"):
            self.assertEqual(self.cm.classify(args), "codex")
        self.assertEqual(self.cm.classify("codex-helper"), "cmd")

    def test_codex_launch_uses_native_flags(self):
        parts = shlex.split(self.cm.launch_line("codex", "", "example", "high", "task"))
        self.assertEqual(parts, ["codex", "-c", self.cm.CODEX_TITLE_CONFIG, "--model", "example",
                                 "-c", 'model_reasoning_effort="high"'])
        self.assertNotIn("--session-id", parts)
        self.assertNotIn("--name", parts)
        self.assertEqual(shlex.split(self.cm.resume_line("codex", SID))[:3], ["codex", "resume", SID])

    def test_generic_shell_command_is_preserved(self):
        command = "printf '%s\\n' 'space and ; $literal' | tee result; sleep 60"
        self.assertEqual(shlex.split(self.cm.launch_line("command", "", command=command)),
                         ["/bin/sh", "-c", command])
        with self.assertRaisesRegex(self.cm.CmError, "no conversation resume"):
            self.cm.resume_line("command", SID)

    def test_full_native_title(self):
        self.assertEqual(self.cm.codex_pane_id({"title": SID, "cwd": "/work"}), SID)
        for title in ("Discuss " + SID, SID + " trailing", "codex", "", "1111..."):
            self.assertIsNone(self.cm.codex_pane_id({"title": title, "cwd": "/work"}))

    def test_native_prefix_resolves_only_one_persisted_id(self):
        self.index([(SID, "/work"), (OTHER, "/other")])
        self.assertEqual(self.cm.codex_pane_id({"title": PREFIX, "cwd": "/work"}), SID)
        self.assertIsNone(self.cm.codex_pane_id({"title": PREFIX, "cwd": "/missing"}))

    def test_ambiguous_prefix_is_not_guessed(self):
        self.index([(SID, "/work"), (OTHER, "/work")])
        self.assertIsNone(self.cm.codex_pane_id({"title": PREFIX, "cwd": "/work", "managed_id": SID}))

    def test_rollout_fallback(self):
        d = self.home / "sessions" / "year"
        d.mkdir(parents=True)
        path = d / f"rollout-date-{SID}.jsonl"
        path.write_text(json.dumps({"type": "session_meta", "payload": {"id": SID, "cwd": "/work"}}) + "\n")
        self.assertEqual(self.cm.codex_pane_id({"title": PREFIX, "cwd": "/work"}), SID)
        self.assertEqual(self.cm.transcript_files("codex", "/work"), [(path, SID)])
        self.assertEqual(self.cm.transcript_files("codex", "/other"), [])

    def test_unknown_title_cannot_use_newest_session(self):
        self.index([(SID, "/work")])
        self.assertIsNone(self.cm.codex_pane_id({"title": "codex", "cwd": "/work"}))

    def test_invalid_input_fails_before_creating_tmux(self):
        self.cm.tmux = lambda *args, **kw: self.fail("must validate before tmux")
        invalid = [
            ["--command", ""], ["--command", "echo a\necho b"],
            ["--command", "sleep 60", "--model", "example"],
            ["--command", "sleep 60", "--harness", "codex"],
            ["--harness", "pi", "--timeout", "nan"],
            ["--harness", "pi", "--timeout", "0"],
        ]
        for args in invalid:
            with self.subTest(args=args), contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
                self.cm.main(["spawn", "--id", "task", "--cwd", str(self.root), *args])
            self.assertEqual(error.exception.code, 2)

    def test_rebuild_rejects_declared_agent_command_without_id(self):
        self.cm.REGISTRY.parent.mkdir(parents=True)
        recovery = self.root / "resume.md"
        self.cm.has_session = lambda _: False
        self.cm.tmux = lambda *args, **kw: self.fail("must reject before creating tmux panes")
        for harness in self.cm.AGENTS:
            recovery.write_text(f"# Resume state: task\n\n## window 1: task\n### pane 0\ncwd: {self.root}\n"
                                f"harness: {harness}\ncommand: {harness}\n")
            self.cm.REGISTRY.write_text(f"# Sessions\n\n## task\nharness: {harness}\nresume_state: {recovery}\nshutdown: now\n")
            with self.subTest(harness=harness), contextlib.redirect_stderr(io.StringIO()) as err, self.assertRaises(SystemExit):
                self.cm.main(["rebuild", "task"])
            self.assertIn("command has no session_id", err.getvalue())
            self.assertIsNone(self.cm.Registry.load().entry("task").get("tmux_session"))

    def test_native_session_switch_replaces_registry_identity(self):
        self.cm.REGISTRY.parent.mkdir(parents=True)
        self.cm.REGISTRY.write_text(f"# Sessions\n\n## task\nharness: codex\ntmux_session: task\nresumed_session_id: {OTHER}\n")
        self.cm.pane_rows = lambda target=None: [dict(session="task", window=1, pane=0, pane_id="%1",
            pane_pid=10, tty="t", cwd="/work", title=SID, managed_harness="codex")]
        self.cm.ps_table = lambda: [(10, 1, "t", "codex")]
        self.cm.store_by_pane = lambda _: {}
        self.cm.has_session = lambda _: True
        self.assertEqual(self.cm.record_pane_ids("task"), {"%1": SID})
        self.assertEqual(self.cm.Registry.load().entry("task").get("resumed_session_id"), SID)

    def test_codex_cannot_inherit_worker_id_by_cwd(self):
        self.cm.REGISTRY.parent.mkdir(parents=True)
        self.cm.REGISTRY.write_text(f"# Sessions\n\n## task\nharness: codex\ntmux_session: task\nworker: {SID} cwd=/work\n")
        self.cm.pane_rows = lambda target=None: [dict(session="task", window=1, pane=1, pane_id="%2",
            pane_pid=10, tty="t", cwd="/work", title="codex", managed_harness="codex")]
        self.cm.ps_table = lambda: [(10, 1, "t", "codex")]
        self.cm.store_by_pane = lambda _: {}
        self.assertIsNone(self.cm.inspect_panes("task")[0]["session_id"])

    def test_unresolved_new_title_does_not_reuse_old_registry_id(self):
        self.cm.REGISTRY.parent.mkdir(parents=True)
        self.cm.REGISTRY.write_text(f"# Sessions\n\n## task\nharness: codex\ntmux_session: task\nresumed_session_id: {SID}\n")
        self.cm.pane_rows = lambda target=None: [dict(session="task", window=1, pane=0, pane_id="%1",
            pane_pid=10, tty="t", cwd="/work", title=PREFIX.replace("11111111", "aaaaaaaa"), managed_harness="codex")]
        self.cm.ps_table = lambda: [(10, 1, "t", "codex")]
        self.cm.store_by_pane = lambda _: {}
        self.assertIsNone(self.cm.inspect_panes("task")[0]["session_id"])
        self.cm.has_session = lambda _: True
        self.cm.record_pane_ids("task")
        self.assertIsNone(self.cm.Registry.load().entry("task").get("resumed_session_id"))


class TmuxHarnessTests(test_startup.IsolatedTmuxCase):
    def test_generic_lifecycle_preserves_shell_semantics(self):
        marker = self.root / "marker with spaces"
        command = f"printf '%s' 'a; $literal' > {shlex.quote(str(marker))}; sleep 60"
        self.cm.main(["spawn", "--id", "generic", "--cwd", str(self.root), "--command", command])
        self.assertEqual(marker.read_text(), "a; $literal")
        entry = self.cm.Registry.load().entry("generic")
        self.assertEqual(entry.get("harness"), "command")
        self.assertIsNone(entry.get("resumed_session_id"))
        self.cm.main(["pause", "generic", "on"])
        self.cm.main(["pause", "generic", "off"])
        self.cm.main(["shutdown", "generic"])
        marker.unlink()
        self.cm.main(["rebuild", "generic"])
        self.assertEqual(marker.read_text(), "a; $literal")
        pane = self.cm.inspect_panes("generic")[0]
        self.assertEqual(pane["command"], command)
        self.assertEqual(pane["kind"], "cmd")
        self.cm.main(["wrap", "generic"])
        self.assertFalse(self.cm.has_session("generic"))
        self.assertIsNone(self.cm.Registry.load().entry("generic"))

    def test_generic_restart_uses_launch_cwd_after_command_changes_directory(self):
        (self.root / "child").mkdir()
        self.cm.main(["spawn", "--id", "generic", "--cwd", str(self.root),
                      "--command", "cd child && exec sleep 60"])
        self.assertEqual(self.cm.pane_rows("generic")[0]["cwd"], str(self.root / "child"))
        self.cm.main(["shutdown", "generic"])
        path = self.cm.Registry.load().entry("generic").get("resume_state")
        self.assertEqual(self.cm.parse_resume_state(Path(path).read_text())["windows"][0]["panes"][0]["cwd"], str(self.root))
        self.cm.main(["rebuild", "generic"])
        self.assertFalse(self.cm.inspect_panes("generic")[0]["dead"])
        self.assertEqual(self.cm.pane_rows("generic")[0]["cwd"], str(self.root / "child"))

    def test_worker_identity_can_be_corrected_without_hooks(self):
        self.spawn()
        self.cm.main(["spawn", "--into", "work", "--label", "second"])
        pane = self.cm.inspect_panes("work")[1]
        old_id = pane["session_id"]
        self.cm.store_by_pane = lambda _: {pane["pane_id"]: (old_id, "working")}
        self.cm.main(["identify", "work", "--pane", pane["pane_id"], "--session-id", "corrected-id"])
        self.assertEqual(self.cm.inspect_panes("work")[1]["session_id"], "corrected-id")
        workers = self.cm.Registry.load().entry("work").get_all("worker")
        self.assertEqual(workers, [f"corrected-id cwd={self.root} label=second"])
        _, missing, panes = self.cm.write_resume_state("work", self.root / "resume.md")
        self.assertFalse(missing)
        self.assertEqual(panes[1]["session_id"], "corrected-id")
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            self.cm.main(["identify", "work", "--pane", "%9999", "--session-id", "wrong"])
        self.assertEqual(self.cm.Registry.load().entry("work").get_all("worker"), workers)

    def test_generic_pane_inside_agent_session(self):
        self.spawn()
        self.cm.main(["spawn", "--into", "work", "--label", "server", "--command", "sleep 60"])
        panes = self.cm.inspect_panes("work")
        self.assertEqual([p["kind"] for p in panes], ["pi", "cmd"])
        self.assertEqual(panes[1]["label"], "server")
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            self.cm.main(["spawn", "--into", "work", "--label", "server", "--command", "sleep 60"])
        with contextlib.redirect_stdout(io.StringIO()) as output:
            self.cm.main(["reconcile"])
        self.assertNotIn("mixed", output.getvalue())
        path, missing, _ = self.cm.write_resume_state("work", self.root / "resume.md")
        self.assertFalse(missing)
        self.assertIn("harness: command\nlabel: server", path.read_text())

    def test_codex_identity_and_resume_without_hooks(self):
        codex = self.bin / "codex"
        codex.write_text(f"#!/bin/sh\nprintf '\\033]0;{SID}\\007'\nexec sleep 60\n")
        codex.chmod(0o755)
        self.cm.main(["spawn", "--harness", "codex", "--id", "code", "--cwd", str(self.root)])
        self.assertEqual(self.cm.Registry.load().entry("code").get("resumed_session_id"), SID)
        self.cm.main(["shutdown", "code"])
        self.cm.main(["rebuild", "code"])
        pane = self.cm.inspect_panes("code")[0]
        self.assertEqual(pane["session_id"], SID)
        self.assertEqual(pane["kind"], "codex")
        self.assertFalse(pane["dead"])

    def test_codex_without_native_id_is_reported_not_invented(self):
        codex = self.bin / "codex"
        codex.write_text("#!/bin/sh\nexec sleep 60\n")
        codex.chmod(0o755)
        with contextlib.redirect_stdout(io.StringIO()) as out, contextlib.redirect_stderr(io.StringIO()) as err:
            self.cm.main(["spawn", "--harness", "codex", "--id", "code", "--cwd", str(self.root)])
        self.assertIn("session_id=unknown", out.getvalue())
        self.assertIn("not available", err.getvalue())
        self.assertIsNone(self.cm.Registry.load().entry("code").get("resumed_session_id"))
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            self.cm.main(["shutdown", "code"])
        self.assertEqual(error.exception.code, 2)
        self.assertTrue(self.cm.has_session("code"))
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            self.cm.main(["identify", "code"])
        self.assertEqual(error.exception.code, 3)

    def test_codex_panes_in_same_cwd_keep_distinct_ids(self):
        codex = self.bin / "codex"
        codex.write_text(f"#!/bin/sh\ncase \"$TMUX_PANE\" in %1) sid={SID};; *) sid={OTHER};; esac\nprintf '\\033]0;%s\\007' \"$sid\"\nexec sleep 60\n")
        codex.chmod(0o755)
        self.cm.main(["spawn", "--harness", "codex", "--id", "code", "--cwd", str(self.root)])
        self.cm.main(["spawn", "--into", "code", "--label", "second"])
        entry = self.cm.Registry.load().entry("code")
        self.assertEqual(entry.get("resumed_session_id"), SID)
        self.assertEqual(entry.get_all("worker"), [f"{OTHER} cwd={self.root} label=second"])

    def test_failed_rebuild_preserves_recovery_and_error_output(self):
        script = self.bin / "worker"
        script.write_text("#!/bin/sh\nexec sleep 60\n")
        script.chmod(0o755)
        self.cm.main(["spawn", "--id", "generic", "--cwd", str(self.root), "--command", str(script)])
        self.cm.main(["shutdown", "generic"])
        recovery = self.cm.Registry.load().entry("generic").get("resume_state")
        before = Path(recovery).read_text()
        script.write_text("#!/bin/sh\necho 'cannot restart'\nexit 7\n")
        with contextlib.redirect_stderr(io.StringIO()) as err, self.assertRaises(SystemExit):
            self.cm.main(["rebuild", "generic"])
        self.assertIn("status 7", err.getvalue())
        self.assertIn("failed rebuild output", err.getvalue())
        self.assertFalse(self.cm.has_session("generic"))
        self.assertEqual(Path(recovery).read_text(), before)
        self.assertIsNotNone(self.cm.Registry.load().entry("generic").get("shutdown"))
        self.assertIn("cannot restart", (self.cm.STATE / "snapshots/generic-rebuild-failed.txt").read_text())

    def test_mixed_agent_spawn_is_rejected(self):
        self.spawn()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            self.cm.main(["spawn", "--into", "work", "--harness", "codex", "--label", "wrong"])
        self.assertEqual(len(self.cm.pane_rows("work")), 1)
