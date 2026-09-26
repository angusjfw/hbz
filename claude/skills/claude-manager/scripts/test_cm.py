"""Tests for cm's registry and file handling. Run: python3 -m unittest (from scripts/)."""

import importlib.machinery
import importlib.util
import json
import os
import shutil
import tempfile
import threading
import time
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
LIVE_STATE = Path.home() / ".local/state/claude-manager"


def load_cm(state_dir):
    os.environ["CM_STATE_DIR"] = str(state_dir)
    loader = importlib.machinery.SourceFileLoader("cm", str(HERE / "cm"))
    spec = importlib.util.spec_from_loader("cm", loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


SAMPLE = """# Sessions

manager: 0:1.0 harness=claude

Some stray prose in the header.

## alpha
ticket: ENG-1
tmux_session: alpha
custom_field: keep me
resumed_session_id: 11111111-1111-1111-1111-111111111111
worker: 22222222-2222-2222-2222-222222222222 cwd=~/x label=api
notes: long note: with colons: inside

Prose paragraph under alpha.
- a bullet: that looks like a field but isn't lowercase-keyed

## beta
harness: pi
shutdown: 2026-09-01
"""


class RegistryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.cm = load_cm(self.tmp)
        (self.tmp / "sessions.md").write_text(SAMPLE)

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def run_cm(self, *argv):
        self.cm.main(list(argv))

    def text(self):
        return (self.tmp / "sessions.md").read_text()

    def test_round_trip_is_identity(self):
        reg = self.cm.Registry(SAMPLE)
        self.assertEqual(reg.text(), SAMPLE)

    def test_set_replaces_in_place_and_keeps_everything_else(self):
        self.run_cm("reg", "set", "alpha", "ticket=ENG-2")
        self.assertEqual(self.text(), SAMPLE.replace("ticket: ENG-1", "ticket: ENG-2"))

    def test_set_new_field_goes_after_last_field_before_prose(self):
        self.run_cm("reg", "set", "alpha", "paused=2026-09-26")
        t = self.text()
        self.assertIn("notes: long note: with colons: inside\npaused: 2026-09-26\n\nProse paragraph", t)
        self.assertIn("custom_field: keep me", t)
        self.assertIn("Some stray prose in the header.", t)

    def test_now_uses_clock(self):
        self.run_cm("reg", "set", "beta", "last_touched=now")
        self.assertRegex(self.text(), r"last_touched: \d{4}-\d\d-\d\d \d\d:\d\d")

    def test_unset_and_match(self):
        self.run_cm("reg", "unset", "alpha", "tmux_session")
        self.assertNotIn("tmux_session: alpha", self.text())
        self.run_cm("reg", "worker", "drop", "alpha", "22222222-2222-2222-2222-222222222222")
        self.assertNotIn("worker:", self.text())

    def test_worker_add_is_idempotent(self):
        self.run_cm("reg", "worker", "add", "alpha", "3333", "cwd=/y", "label=ui")
        self.run_cm("reg", "worker", "add", "alpha", "3333", "cwd=/y", "label=ui")
        t = self.text()
        self.assertEqual(t.count("worker: 3333 cwd=/y label=ui"), 1)
        self.assertLess(t.index("worker: 2222"), t.index("worker: 3333"))

    def test_header_append_and_unset(self):
        self.run_cm("reg", "set", "@header", "manager+=pi-manager:1.0 harness=pi")
        t = self.text()
        self.assertIn("manager: 0:1.0 harness=claude\nmanager: pi-manager:1.0 harness=pi\n", t)
        self.run_cm("reg", "unset", "@header", "manager=0:1.0")
        self.assertNotIn("manager: 0:1.0", self.text())
        self.assertIn("manager: pi-manager:1.0", self.text())

    def test_header_insert_when_empty(self):
        (self.tmp / "sessions.md").write_text("# Sessions\n\n## a\ncwd: /x\n")
        self.run_cm("reg", "set", "@header", "manager+=m:1.0 harness=claude")
        self.assertEqual(self.text(), "# Sessions\n\nmanager: m:1.0 harness=claude\n\n## a\ncwd: /x\n")

    def test_new_and_rm(self):
        self.run_cm("reg", "new", "gamma", "harness=claude", "cwd=/z")
        reg = self.cm.Registry.load()
        g = reg.entry("gamma")
        self.assertEqual(g.get("cwd"), "/z")
        self.assertIsNotNone(g.get("started"))
        self.run_cm("reg", "rm", "gamma")
        self.assertEqual(self.text(), SAMPLE)

    def test_new_rejects_duplicate(self):
        with self.assertRaises(SystemExit):
            self.run_cm("reg", "new", "alpha")

    def test_legacy_entry_defaults_to_claude(self):
        reg = self.cm.Registry(SAMPLE)
        self.assertEqual(self.cm.harness_of(reg.entry("alpha")), "claude")
        self.assertEqual(self.cm.harness_of(reg.entry("beta")), "pi")

    def test_newline_in_value_rejected(self):
        with self.assertRaises(SystemExit):
            self.run_cm("reg", "set", "alpha", "notes=a\nb")

    def test_last_writer_recorded(self):
        os.environ["TMUX_PANE"] = "%99"
        try:
            self.run_cm("reg", "set", "alpha", "ticket=X")
        finally:
            del os.environ["TMUX_PANE"]
        self.assertEqual((self.tmp / ".last-writer").read_text().strip(), "%99")


class LockTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.cm = load_cm(self.tmp)
        (self.tmp / "sessions.md").write_text(SAMPLE)

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def test_held_lock_blocks_then_times_out(self):
        os.mkdir(self.tmp / "sessions.md.lock")
        start = time.time()
        with self.assertRaises(self.cm.CmError):
            self.cm.acquire_lock(timeout=0.5)
        self.assertGreaterEqual(time.time() - start, 0.5)

    def test_writer_waits_for_release(self):
        os.mkdir(self.tmp / "sessions.md.lock")
        threading.Timer(0.3, lambda: os.rmdir(self.tmp / "sessions.md.lock")).start()
        self.cm.main(["reg", "set", "alpha", "ticket=W"])
        self.assertIn("ticket: W", (self.tmp / "sessions.md").read_text())
        self.assertFalse((self.tmp / "sessions.md.lock").exists())

    def test_lock_released_on_error(self):
        with self.assertRaises(SystemExit):
            self.cm.main(["reg", "set", "nope", "a=b"])
        self.assertFalse((self.tmp / "sessions.md.lock").exists())

    def test_concurrent_writers_do_not_clobber(self):
        def w(i):
            self.cm.main(["reg", "worker", "add", "alpha", f"id{i}"])
        ts = [threading.Thread(target=w, args=(i,)) for i in range(8)]
        for t in ts:
            t.start()
        for t in ts:
            t.join()
        text = (self.tmp / "sessions.md").read_text()
        for i in range(8):
            self.assertIn(f"worker: id{i}", text)


LEGACY_RESUME = """# Resume state: demo

shutdown: 2026-05-22

Some prose about the state.

## window 1: claude
layout: 5fe4,200x50,0,0,0

### pane 0
cwd: ~/code/demo
command: claude --effort high --resume abc-123
claude_session_id: abc-123

## window 2: dev
layout: 9a3c,200x50,0,0{100x50,0,0,1,99x50,101,0,2}

### pane 0
cwd: ~/code/mock
command: yarn mock

### pane 1
cwd: ~/code/demo
command:

## Where the work stands

Prose with a field-like line:
note: not a pane field
"""


class ResumeStateTests(unittest.TestCase):
    def setUp(self):
        self.cm = load_cm(Path(tempfile.mkdtemp()))

    def test_legacy_parse(self):
        rs = self.cm.parse_resume_state(LEGACY_RESUME)
        self.assertEqual(rs["fields"]["shutdown"], "2026-05-22")
        self.assertEqual([w["name"] for w in rs["windows"]], ["claude", "dev"])
        p0 = rs["windows"][0]["panes"][0]
        self.assertEqual(p0["session_id"], "abc-123")
        self.assertEqual(p0["harness"], "claude")
        dev = rs["windows"][1]
        self.assertEqual(dev["layout"], "9a3c,200x50,0,0{100x50,0,0,1,99x50,101,0,2}")
        self.assertEqual(dev["panes"][0]["command"], "yarn mock")
        self.assertEqual(dev["panes"][1]["command"], "")
        self.assertNotIn("note", dev["panes"][1])

    def test_live_files_parse(self):
        """A copy of the live registry and resume files, if present, parses and round-trips."""
        if not (LIVE_STATE / "sessions.md").exists():
            self.skipTest("no live registry")
        tmp = Path(tempfile.mkdtemp())
        try:
            shutil.copy(LIVE_STATE / "sessions.md", tmp / "sessions.md")
            text = (tmp / "sessions.md").read_text()
            self.assertEqual(self.cm.Registry(text).text(), text)
            for f in (LIVE_STATE / "resume").glob("*.md"):
                shutil.copy(f, tmp / f.name)
                rs = self.cm.parse_resume_state((tmp / f.name).read_text())
                self.assertTrue(rs["windows"], f.name)
                for w in rs["windows"]:
                    self.assertTrue(w["panes"], f"{f.name} window {w['index']}")
        finally:
            shutil.rmtree(tmp)


class TranscriptTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.cm = load_cm(self.tmp / "state")
        os.environ["PI_CODING_AGENT_SESSION_DIR"] = str(self.tmp / "pi")

    def tearDown(self):
        del os.environ["PI_CODING_AGENT_SESSION_DIR"]
        shutil.rmtree(self.tmp)

    def test_pi_slug(self):
        self.assertEqual(self.cm.pi_slug("/home/a/my.repo_x"), "--home-a-my.repo_x--")

    def test_claude_dir(self):
        self.assertEqual(self.cm.claude_project_dir("/Users/foo.bar/code/my_service").name,
                         "-Users-foo-bar-code-my-service")

    def test_pi_filters_on_header_cwd(self):
        # /a-b/c and /a/b-c share a slug; only the header tells them apart
        d = self.tmp / "pi" / self.cm.pi_slug("/a-b/c")
        d.mkdir(parents=True)
        (d / "2026_s1.jsonl").write_text(json.dumps({"type": "session", "id": "s1", "cwd": "/a-b/c"}) + "\n"
                                         + json.dumps({"type": "session_info", "name": "w1"}) + "\n")
        (d / "2026_s2.jsonl").write_text(json.dumps({"type": "session", "id": "s2", "cwd": "/a/b-c"}) + "\n")
        got = {sid for _, sid in self.cm.transcript_files("pi", "/a-b/c")}
        self.assertEqual(got, {"s1"})
        self.assertIsNotNone(self.cm.pi_transcript("/a-b/c", "s1"))
        self.assertIsNone(self.cm.pi_transcript("/a-b/c", "s2"))
        self.assertEqual(self.cm.pi_session_names(d / "2026_s1.jsonl"), ["w1"])


class AdapterTests(unittest.TestCase):
    def setUp(self):
        self.cm = load_cm(Path(tempfile.mkdtemp()))

    def test_classify(self):
        c = self.cm.classify
        self.assertEqual(c(""), "idle")
        self.assertEqual(c("claude --resume x"), "claude")
        self.assertEqual(c("node /usr/lib/node_modules/@anthropic-ai/claude-code/cli.js"), "claude")
        self.assertEqual(c("pi"), "pi")
        self.assertEqual(c("pi                    "), "pi")
        self.assertEqual(c("nvim ."), "cmd")
        self.assertEqual(c("pip install x"), "cmd")
        self.assertEqual(c("python pipeline.py"), "cmd")

    def test_lines(self):
        self.assertEqual(self.cm.launch_line("pi", "u1", None, "low", "e-w", "/b f.md"),
                         "pi --session-control --session-id u1 --thinking low --name e-w \"$(cat '/b f.md')\"")
        self.assertEqual(self.cm.resume_line("claude", "u1", "high"), "claude --effort high --resume u1")
        self.assertEqual(self.cm.resume_line("pi", "u1", "high"), "pi --session-control --session-id u1")


if __name__ == "__main__":
    unittest.main()
