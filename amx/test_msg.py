"""amx msg: route to native tools, pi's control socket, or the pane."""

import json
import tempfile
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


if __name__ == "__main__":
    unittest.main()
