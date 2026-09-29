"""amx msg: route to native tools, pi's control socket, or the pane."""

import json
import socket
import tempfile
import threading
import time
import unittest
from pathlib import Path
from unittest.mock import patch

from test_amx import load_cm
import test_startup

SID = "11111111-2222-3333-4444-555555555555"


def pane(harness, name="w", pane_id="%2", sid=SID, state="idle", dead=False):
    return dict(name=name, pane_id=pane_id, harness=harness, session_id=sid, state=state, dead=dead)


class RouteTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cm = load_cm(Path(self.tmp.name))

    def route(self, sender, target, via="auto", live=False):
        return self.cm.msg_route(sender, target, via, live)

    def refused(self, reason, *args, **kw):
        with self.assertRaises(self.cm.Refused) as cm:
            self.route(*args, **kw)
        self.assertEqual(cm.exception.reason, reason)
        return str(cm.exception)

    def test_cross_harness_routes(self):
        claude, pi, codex = pane("claude", "c", "%1"), pane("pi"), pane("codex")
        self.assertEqual(self.route(claude, pi, live=True), "socket")
        self.assertEqual(self.route(claude, pi), "pane")
        self.assertEqual(self.route(pi, claude), "pane")
        self.assertEqual(self.route(codex, pane("pi", pane_id="%3"), live=True), "socket")
        self.assertEqual(self.route(claude, codex), "pane")
        self.assertEqual(self.route(pane("codex", pane_id="%9"), codex), "pane")
        self.assertEqual(self.route(None, pane("claude")), "pane")

    def test_native_pairs_are_refused_with_the_tool(self):
        self.assertIn("SendMessage", self.refused("native", pane("claude", pane_id="%1"), pane("claude")))
        self.assertIn("send_to_session", self.refused("native", pane("pi", pane_id="%1"), pane("pi"), live=True))
        self.assertEqual(self.route(pane("pi", pane_id="%1"), pane("pi")), "pane")

    def test_forced_routes(self):
        claude = pane("claude", pane_id="%1")
        self.assertEqual(self.route(claude, pane("claude"), via="pane"), "pane")
        self.assertEqual(self.route(pane("pi", pane_id="%1"), pane("pi"), via="socket", live=True), "socket")
        self.refused("no-socket", claude, pane("pi"), via="socket")
        self.refused("no-socket", claude, pane("claude"), via="socket")

    def test_unreachable_targets(self):
        self.refused("dead", None, pane("claude", dead=True))
        self.assertIn("tmux-interaction", self.refused("command", None, pane("command")))
        self.refused("command", None, pane("command"), via="pane")

    def test_header_names_the_reply_route(self):
        self.assertEqual(self.cm.msg_header(pane("claude", "work-api", "%5")),
                         '[from work-api (claude, %5); reply: amx msg work-api "..."]')
        self.assertEqual(self.cm.msg_header(pane("pi", "%7", "%7")),
                         '[from %7 (pi, %7); reply: amx msg %7 "..."]')
        self.assertEqual(self.cm.msg_header(None), "[from an agent outside tmux; no reply route]")

    def test_compose(self):
        claude, pi = pane("claude", "c", "%1"), pane("pi", "p", "%1")
        self.assertEqual(self.cm.compose(claude, "socket", "hi"), self.cm.msg_header(claude) + "\nhi")
        self.assertEqual(self.cm.compose(pi, "pane", "hi"), self.cm.msg_header(pi) + "\nhi")
        body, tag = self.cm.compose(pi, "socket", "hi").split("\n\n")
        self.assertEqual(body, "hi")
        info = json.loads(tag.removeprefix("<sender_info>").removesuffix("</sender_info>"))
        self.assertEqual(info, {"sessionId": SID})


class FakeControl:
    """A session-control socket that records one command and answers it."""

    def __init__(self, path, reply):
        self.path, self.reply, self.got = path, reply, None
        self.server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.server.bind(str(path))
        self.server.listen(1)
        self.thread = threading.Thread(target=self.serve, daemon=True)
        self.thread.start()

    def serve(self):
        while self.got is None:  # a liveness probe connects and hangs up; wait for a command
            conn, _ = self.server.accept()
            with conn:
                buf = b""
                while b"\n" not in buf and (chunk := conn.recv(4096)):
                    buf += chunk
                if not buf:
                    continue
                self.got = json.loads(buf)
                if self.reply is not None:
                    conn.sendall(self.reply)

    def close(self):
        self.thread.join(2)
        self.server.close()


class SocketTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cm = load_cm(Path(self.tmp.name) / "state")
        self.path = Path(self.tmp.name) / "s.sock"

    def serve(self, reply):
        fake = FakeControl(self.path, reply)
        self.addCleanup(fake.close)
        return fake

    def test_send_and_read_the_response(self):
        fake = self.serve(b'{"type":"response","command":"send","success":true}\n')
        self.assertTrue(self.cm.socket_answers(self.path))
        resp = self.cm.socket_send(self.path, "hi \"there\"\n$x", "follow_up")
        fake.close()
        self.assertEqual(fake.got, {"type": "send", "message": "hi \"there\"\n$x", "mode": "follow_up"})
        self.assertTrue(resp["success"])

    def test_no_listener(self):
        self.assertIsNone(self.cm.socket_send(self.path, "hi", "steer"))
        self.assertFalse(self.cm.socket_answers(self.path))
        self.path.touch()  # a stale socket file left by a killed pi
        self.assertIsNone(self.cm.socket_send(self.path, "hi", "steer"))
        self.assertFalse(self.cm.socket_answers(self.path))

    def test_closed_without_reply_is_an_error(self):
        self.serve(None)
        with self.assertRaisesRegex(self.cm.CmError, "no reply"):
            self.cm.socket_send(self.path, "hi", "steer")

    def test_socket_path_uses_the_native_id(self):
        self.assertEqual(self.cm.pi_socket(SID), self.cm.PI_CONTROL / f"{SID}.sock")


RULE = "─" * 60
PI_EMPTY = f"""
 ok3
{RULE}

{RULE} INSERT
~/work · main · pi-worker · model · medium
0.0%/272k (auto) · session x
"""
PI_DRAFT = PI_EMPTY.replace(f"{RULE}\n\n{RULE}", f"{RULE}\nhello draft\n  second line\n{RULE}")
CLAUDE_INSERT = f"""
❯ hi
{RULE}
  ~/work · main · 0% · Haiku 4.5 · medium
  -- INSERT -- ⏸ manual mode on
"""


class ReadinessTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cm = load_cm(Path(self.tmp.name))
        self.clock = 0
        self.addCleanup(patch.stopall)
        patch.object(self.cm.time, "monotonic", lambda: self.clock).start()
        patch.object(self.cm.time, "sleep", self.sleep).start()
        self.box = "empty"
        self.cm.box_state = lambda target: self.box
        self.states = []

    def sleep(self, seconds):
        self.clock += seconds

    def resolve(self, _):
        return pane("claude", state=self.states.pop(0))

    def ready(self, target, wait=0, force=False):
        self.cm.resolve_target = self.resolve
        return self.cm.ready_for_typing(target, wait, force)

    def refused(self, reason, *args, **kw):
        with self.assertRaises(self.cm.Refused) as cm:
            self.ready(*args, **kw)
        self.assertEqual(cm.exception.reason, reason)

    def test_pi_box(self):
        self.assertEqual(self.cm.pi_box(PI_EMPTY), "empty")
        self.assertEqual(self.cm.pi_box(PI_DRAFT), "draft")
        self.assertEqual(self.cm.pi_box("a dialog\nno editor here\n"), "no-box")

    def test_insert_mode(self):
        self.assertTrue(self.cm.insert_mode(CLAUDE_INSERT))
        self.assertTrue(self.cm.insert_mode(PI_EMPTY))
        self.assertFalse(self.cm.insert_mode(PI_EMPTY.replace("INSERT", "NORMAL")))
        self.assertFalse(self.cm.insert_mode(CLAUDE_INSERT.replace("-- INSERT --", "")))
        self.assertFalse(self.cm.insert_mode("the word INSERT in output\n" + RULE + "\n"))

    def test_idle_and_done_are_ready(self):
        for state in ("idle", "done", "error"):
            self.assertEqual(self.ready(pane("claude", state=state))["state"], state)

    def test_busy(self):
        self.refused("busy", pane("claude", state="working"))
        self.states = ["working", "working", "idle"]
        self.assertEqual(self.ready(pane("claude", state="working"), wait=5)["state"], "idle")
        self.states = ["working"] * 10
        self.refused("busy", pane("claude", state="working"), wait=3)
        self.assertGreaterEqual(self.clock, 3)

    def test_force_over_a_stale_working_state(self):
        self.assertEqual(self.ready(pane("claude", state="working"), force=True)["state"], "working")
        self.box = "draft"
        self.refused("draft", pane("claude", state="working"), force=True)

    def test_waiting_on_the_user_is_never_forced(self):
        for box in ("no-box", None):
            self.box = box
            self.refused("needs-input", pane("claude", state="needs_input"), force=True)

    def test_needs_input_outlived_by_a_cancelled_prompt(self):
        self.box = "empty"
        self.assertEqual(self.ready(pane("claude", state="needs_input"))["state"], "needs_input")

    def test_unknown_state_needs_force(self):
        for state in (None, "off"):
            self.refused("state-unknown", pane("claude", state=state))
            self.assertEqual(self.ready(pane("claude", state=state), force=True)["state"], state)

    def test_input_box(self):
        self.box = "ghost"
        self.ready(pane("claude"))
        for box in ("draft", "no-box"):
            self.box = box
            self.refused("draft" if box == "draft" else "box-unknown", pane("claude"), force=True)
        self.box = None
        self.refused("box-unknown", pane("codex"))
        self.ready(pane("codex"), force=True)


class TypingTests(test_startup.IsolatedTmuxCase):
    """A stub agent in raw mode records every byte a message sends."""

    def stub(self, banner):
        out = self.root / "received"
        script = self.root / "stub"
        script.write_text(f"#!/bin/sh\nprintf '\\033[?2004h{banner}\\n'\nstty raw -echo\nexec cat > {out}\n")
        script.chmod(0o755)
        pane_id = self.tmux("new-session", "-d", "-s", "stub", "-P", "-F", "#{pane_id}", str(script)).stdout.strip()
        deadline = time.monotonic() + 5
        while not out.exists() and time.monotonic() < deadline:
            time.sleep(0.1)
        return pane("claude", pane_id=pane_id), out

    def received(self, out, until):
        deadline = time.monotonic() + 5
        while until not in out.read_bytes() and time.monotonic() < deadline:
            time.sleep(0.1)
        return out.read_bytes()

    def test_paste_arrives_intact_and_submits(self):
        target, out = self.stub("-- INSERT --")
        self.cm.box_state = lambda t: "empty"
        text = "[from a (pi, %1)]\nquotes \"x\" 'y' $HOME `date` \\n tab\tend"
        self.assertTrue(self.cm.type_into(target, text))
        got = self.received(out, b"\r")
        self.assertEqual(got, b"\x1b[200~" + text.replace("\n", "\r").encode() + b"\x1b[201~\x1b\r")

    def test_no_escape_outside_insert_mode(self):
        target, out = self.stub("")
        self.cm.box_state = lambda t: "draft"
        self.assertFalse(self.cm.type_into(target, "hi"))
        self.assertEqual(self.received(out, b"\r"), b"\x1b[200~hi\x1b[201~\r")


if __name__ == "__main__":
    unittest.main()
