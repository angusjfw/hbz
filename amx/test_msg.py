"""amx msg: route to native tools, pi's control socket, or the pane."""

import json
import socket
import tempfile
import threading
import unittest
from pathlib import Path

from test_amx import load_cm

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


if __name__ == "__main__":
    unittest.main()
