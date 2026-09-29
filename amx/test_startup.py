"""Portable startup observation; no agent hooks or personal config required."""

import contextlib
import io
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from test_amx import load_cm


class StartupTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cm = load_cm(Path(self.tmp.name))
        self.clock = 0
        self.samples = []
        self.cm.tmux = self.tmux
        self.addCleanup(patch.stopall)
        patch.object(self.cm.time, "monotonic", lambda: self.clock).start()
        patch.object(self.cm.time, "sleep", self.sleep).start()

    def sleep(self, seconds):
        self.clock += seconds

    def tmux(self, *args, **kwargs):
        sample = self.samples.pop(0) if len(self.samples) > 1 else self.samples[0]
        return SimpleNamespace(stdout=sample, returncode=0 if sample else 1)

    def test_immediate_exit(self):
        self.samples = ["1\t1\t42\t\n"]
        with self.assertRaisesRegex(self.cm.CmError, "status 1.*retained"):
            self.cm.wait_for_start("%1", 3)

    def test_transient_process_is_not_success(self):
        self.samples = ["0\t\t42\t\n", "1\t2\t42\t\n"]
        with self.assertRaisesRegex(self.cm.CmError, "status 2"):
            self.cm.wait_for_start("%1", 3)

    def test_exit_zero_is_not_a_running_session(self):
        self.samples = ["1\t0\t42\t\n"]
        with self.assertRaisesRegex(self.cm.CmError, "status 0"):
            self.cm.wait_for_start("%1", 3)

    def test_signal_exit(self):
        self.samples = ["1\t\t42\t15\n"]
        with self.assertRaisesRegex(self.cm.CmError, "signal 15"):
            self.cm.wait_for_start("%1", 3)

    def test_delayed_running_without_hooks(self):
        self.samples = ["\t\t\t\n", "0\t\t42\t\n"]
        self.cm.store_by_pane = lambda _: self.fail("startup must not require the status store")
        self.assertEqual(self.cm.wait_for_start("%1", 3), "running")
        self.assertGreaterEqual(self.clock, 2.1)

    def test_stale_status_cannot_hide_failure(self):
        self.samples = ["1\t1\t42\t\n"]
        self.cm.store_by_pane = lambda _: {"%1": ("expected", "working")}
        with self.assertRaises(self.cm.CmError):
            self.cm.wait_for_start("%1", 3)

    def test_timeout(self):
        self.samples = ["0\t\t42\t\n"]
        with self.assertRaisesRegex(self.cm.CmError, "timed out"):
            self.cm.wait_for_start("%1", 0.2)

    def test_missing_pane(self):
        self.samples = [""]
        with self.assertRaisesRegex(self.cm.CmError, "disappeared"):
            self.cm.wait_for_start("%1", 3)

    def test_report_returns_exit_three(self):
        self.samples = ["1\t1\t42\t\n"]
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            self.cm.report_start("%1", 3)
        self.assertEqual(error.exception.code, 3)

    def test_direct_agent_is_not_nested(self):
        self.cm.ps_table = lambda: [(10, 1, "t", "pi"), (20, 10, "t", "helper")]
        with patch.object(self.cm.os, "getppid", return_value=20):
            self.assertTrue(self.cm.sole_agent(10))
        self.cm.ps_table = lambda: [(10, 1, "t", "pi"), (20, 10, "t", "pi")]
        with patch.object(self.cm.os, "getppid", return_value=20):
            self.assertFalse(self.cm.sole_agent(10))


@unittest.skipUnless(os.environ.get("AMX_TMUX_TESTS") == "1" and shutil.which("tmux"),
                     "set AMX_TMUX_TESTS=1 for isolated tmux tests")
class TmuxStartupTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name).resolve()
        self.cm = load_cm(self.root / "state")
        self.socket = f"amx-test-{os.getpid()}-{self.root.name}"
        self.tmux_bin = shutil.which("tmux")
        self.cm.tmux = self.tmux
        self.addCleanup(lambda: self.tmux("kill-server", check=False))
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.agent = self.bin / "pi"
        self.agent.write_text("#!/bin/sh\nexec sleep 60\n")
        self.agent.chmod(0o755)
        env = patch.dict(os.environ, {"PATH": f"{self.bin}:/usr/bin:/bin",
                                     "AMX_AGENT_STATUS_DIR": str(self.root / "agent-status")})
        env.start()
        self.addCleanup(env.stop)
        # A separate server, no user tmux config, no real agents or hooks.
        self.tmux("-f", "/dev/null", "new-session", "-d", "-s", "anchor")
        self.tmux("set-environment", "-g", "PATH", f"{self.bin}:/usr/bin:/bin")
        self.tmux("set-option", "-g", "default-shell", "/bin/sh")
        self.cm.agent_status = lambda *args: None

    def tmux(self, *args, check=True):
        r = subprocess.run([self.tmux_bin, "-L", self.socket, *args], capture_output=True, text=True)
        if check and r.returncode:
            raise self.cm.CmError(r.stderr)
        return r

    def spawn(self, *extra):
        self.cm.main(["spawn", "--harness", "pi", "--id", "work", "--cwd", str(self.root), *extra])

    def test_no_hooks_spawn_and_second_pane(self):
        self.spawn()
        self.cm.main(["spawn", "--into", "work", "--label", "second"])
        panes = self.cm.inspect_panes("work")
        self.assertEqual(len(panes), 2)
        self.assertTrue(all(p["session_id"] for p in panes))
        self.assertFalse(any(p["dead"] for p in panes))
        self.assertEqual(panes[1]["id_source"], "pane")
        self.assertNotIn("--session-control", self.cm.Registry.load().text())

    def test_failure_is_retained_and_reconciled(self):
        self.agent.write_text("#!/bin/sh\necho 'Error: Unknown option: --session-control' >&2\nexit 1\n")
        with self.assertRaises(SystemExit) as error:
            self.spawn()
        self.assertEqual(error.exception.code, 3)
        panes = self.cm.inspect_panes("work")
        self.assertTrue(panes[0]["dead"])
        self.assertEqual(panes[0]["exit_status"], "1")
        output = self.tmux("capture-pane", "-p", "-J", "-S", "-100", "-t", panes[0]["pane_id"]).stdout
        self.assertIn("Unknown option", output)
        with contextlib.redirect_stdout(io.StringIO()) as output:
            self.cm.main(["reconcile"])
        self.assertIn("exited\twork", output.getvalue())

    def test_second_pane_failure(self):
        self.spawn()
        self.agent.write_text("#!/bin/sh\nexit 2\n")
        with self.assertRaises(SystemExit) as error:
            self.cm.main(["spawn", "--into", "work", "--label", "failed"])
        self.assertEqual(error.exception.code, 3)
        self.assertFalse(self.cm.inspect_panes("work")[0]["dead"])
        self.assertTrue(self.cm.inspect_panes("work")[1]["dead"])

    def test_args_survive_shutdown_and_rebuild(self):
        self.spawn("--agent-arg=--no-extensions")
        sid = self.cm.Registry.load().entry("work").get("resumed_session_id")
        # Stand-in transcript for the stand-in agent, in an isolated session root.
        sessions = self.root / "sessions"
        with patch.dict(os.environ, {"PI_CODING_AGENT_SESSION_DIR": str(sessions)}):
            d = sessions / self.cm.pi_slug(str(self.root))
            d.mkdir(parents=True)
            (d / "session.jsonl").write_text(self.cm.json.dumps({"id": sid, "cwd": str(self.root)}) + "\n")
            self.cm.main(["shutdown", "work"])
            self.cm.main(["rebuild", "work"])
        pane = self.cm.inspect_panes("work")[0]
        self.assertEqual(pane["agent_args"], '["--no-extensions"]')
        self.assertFalse(pane["dead"])
