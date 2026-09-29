"""Mixed containers retain pane-local identity, settings and lifecycle boundaries."""

import contextlib
import io
import json
import os
import shlex
import tempfile
import time
import unittest
from pathlib import Path
from unittest.mock import patch

from test_amx import load_cm
from test_startup import IsolatedTmuxCase

SID = "11111111-2222-3333-4444-555555555555"


class MixedIdentityTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cm = load_cm(Path(self.tmp.name))
        self.cm.REGISTRY.write_text("# Sessions\n\n## work\nharness: claude\ntmux_session: work\nresumed_session_id: lead\n")
        self.rows = [self.row("%1", "claude", "lead", primary=True),
                     self.row("%2", "pi", SID, label="review")]
        self.cm.pane_rows = lambda target=None: [r for r in self.rows if not target or r["session"] == target]
        self.cm.ps_table = lambda: []
        self.cm.store_by_pane = lambda _: {}
        self.cm.has_session = lambda session: any(r["session"] == session for r in self.rows)
        self.cm.list_sessions = lambda: sorted({r["session"] for r in self.rows})
        self.cm.sole_agent = lambda _: True
        self.cm.git_branch = lambda _: None
        self.cm.tmux = self.tmux

    def row(self, pane, harness, sid, primary=False, label="", session="work"):
        n = int(pane[1:])
        return dict(session=session, window=1, window_name="work", layout="", pane=n - 1,
                    pane_id=pane, pane_pid=n + 10, tty="t", cwd=self.tmp.name, title="", dead=False,
                    managed_harness=harness, managed_id=sid, managed_primary="1" if primary else "0",
                    managed_label=label, agent_args="[]")

    def tmux(self, *args, **kwargs):
        if args[0] == "set-option":
            row = next(r for r in self.rows if r["pane_id"] == args[3])
            key = {"@amx_session_id": "managed_id", "@amx_harness": "managed_harness",
                   "@amx_primary": "managed_primary", "@amx_label": "managed_label",
                   "@amx_model": "managed_model", "@amx_effort": "managed_effort",
                   "@amx_agent_args": "agent_args"}[args[4]]
            row[key] = args[5]

    def fire(self, pane, harness, event, sid, **extra):
        with patch.dict(os.environ, {"TMUX": "test", "TMUX_PANE": pane}):
            self.cm.handle_hook(harness, dict(hook_event_name=event, session_id=sid, cwd=self.tmp.name, **extra))

    def workers(self):
        return self.cm.worker_records(self.cm.Registry.load().entry("work"))

    def test_legacy_worker_inherits_without_rewriting(self):
        text = self.cm.REGISTRY.read_text() + "worker: old cwd=/work label=old\n"
        self.cm.REGISTRY.write_text(text)
        self.assertEqual(self.workers()[0]["harness"], "claude")
        self.assertEqual(self.cm.REGISTRY.read_text(), text)
        self.cm.main(["reg", "worker", "add", "work", "old", "effort=high"])
        self.assertEqual(self.workers()[0], dict(session_id="old", harness="claude", cwd="/work", label="old", effort="high"))

    def test_legacy_apostrophes_and_backslashes_are_literal(self):
        for cwd in ("/work/o'brien", r"/work/a\backslash"):
            with self.subTest(cwd=cwd):
                legacy = f"old cwd={cwd} label=review"
                worker = self.cm.parse_worker(legacy, "pi")
                self.assertEqual(worker["cwd"], cwd)
                self.assertEqual(self.cm.parse_worker(self.cm.format_worker(worker)), worker)

    def test_truncated_codex_title_without_recorded_id(self):
        self.rows[1].update(managed_harness="codex", managed_id="", title=SID[:29] + "...")
        self.cm.codex_ids = lambda cwd, prefix: {SID}
        self.assertEqual(self.cm.record_pane_ids("work")["%2"], SID)
        self.assertEqual(self.workers()[0]["harness"], "codex")

    def test_harness_switch_does_not_replay_old_adapter_options(self):
        for primary in (False, True):
            with self.subTest(primary=primary):
                row = self.rows[0 if primary else 1]
                row.update(managed_harness="pi", managed_model="pi-model", managed_effort="max",
                           agent_args='["--no-extensions"]')
                if primary:
                    self.cm.main(["reg", "set", "work", "harness=pi", "model=pi-model", "effort=max",
                                  'agent_args=["--no-extensions"]'])
                else:
                    self.fire(row["pane_id"], "pi", "SessionStart", SID)
                self.fire(row["pane_id"], "claude", "SessionStart", "new-primary" if primary else "new-worker")
                saved = self.cm.Registry.load().entry("work").as_dict() if primary else self.workers()[0]
                for key in ("model", "effort", "agent_args"):
                    self.assertNotIn(key, saved)
                resolved = self.cm.resolve_target(row["pane_id"])
                self.assertEqual(resolved["agent_args"], [])
                self.assertIsNone(resolved["effort"])
                self.assertIsNone(resolved["model"])

    def test_explicit_harness_correction_clears_old_pane_options(self):
        self.rows[1].update(managed_model="pi-model", managed_effort="max", agent_args='["--no-extensions"]')
        self.cm.record_pane_ids("work")
        self.cm.ps_table = lambda: [(12, 1, "t", "claude")]
        self.cm.main(["identify", "work", "--pane", "%2", "--session-id", SID])
        resolved = self.cm.resolve_target("%2")
        self.assertEqual(resolved["harness"], "claude")
        self.assertEqual(resolved["agent_args"], [])
        self.assertIsNone(resolved["effort"])
        self.assertEqual(self.workers()[0]["harness"], "claude")

    def test_worker_quoting_and_unknown_fields_round_trip(self):
        worker = dict(session_id=SID, harness="pi", cwd="/work/a b's", label="review",
                      agent_args=json.dumps(["--session-control", "--custom=a b"]), custom="keep me")
        self.assertEqual(self.cm.parse_worker(self.cm.format_worker(worker)), worker)
        self.cm.mutate(lambda reg: self.cm.put_worker(reg.entry("work"), worker))
        self.cm.main(["reg", "worker", "add", "work", SID, "effort=high", "--harness", "pi"])
        self.assertEqual(self.workers()[0], dict(worker, effort="high"))

    def test_same_native_id_is_scoped_by_harness_and_drop_refuses_ambiguity(self):
        for harness in ("claude", "pi", "codex"):
            self.cm.main(["reg", "worker", "add", "work", SID, f"harness={harness}", f"label={harness}"])
        before = self.cm.REGISTRY.read_text()
        for command in (["reg", "worker", "drop", "work", SID], ["reg", "unset", "work", f"worker={SID}"]):
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                self.cm.main(command)
            self.assertEqual(self.cm.REGISTRY.read_text(), before)
        self.cm.main(["reg", "worker", "drop", "work", SID, "--harness", "pi"])
        self.assertEqual([w["harness"] for w in self.workers()], ["claude", "codex"])

    def test_primary_harness_change_keeps_legacy_worker_namespace(self):
        self.cm.REGISTRY.write_text(self.cm.REGISTRY.read_text() + "worker: old cwd=/work\n")
        self.fire("%1", "pi", "SessionStart", "new-lead")
        entry = self.cm.Registry.load().entry("work")
        self.assertEqual(entry.get("harness"), "pi")
        self.assertEqual(self.workers()[0]["harness"], "claude")

    def test_mixed_hook_clear_updates_worker_not_primary(self):
        self.rows[1].update(managed_effort="high", agent_args='["--session-control"]')
        self.fire("%2", "pi", "SessionStart", SID)
        self.fire("%2", "pi", "SessionEnd", SID, reason="clear")
        self.fire("%2", "pi", "SessionStart", "replacement")
        self.assertEqual(self.workers(), [dict(session_id="replacement", harness="pi", cwd=self.tmp.name,
                                              label="review", effort="high", agent_args='["--session-control"]')])
        entry = self.cm.Registry.load().entry("work")
        self.assertEqual((entry.get("harness"), entry.get("resumed_session_id")), ("claude", "lead"))
        self.assertEqual(self.rows[1]["managed_id"], "replacement")
        self.fire("%2", "pi", "SessionEnd", SID, reason="exit")
        self.assertEqual(self.workers()[0]["session_id"], "replacement")
        self.fire("%2", "pi", "SessionEnd", "replacement", reason="exit")
        self.assertEqual(self.workers(), [])

    def test_primary_only_rebuild_refuses_to_discard_worker_recovery(self):
        self.cm.main(["reg", "worker", "add", "work", SID, "harness=pi", "effort=high"])
        self.cm.main(["reg", "unset", "work", "tmux_session"])
        before = self.cm.REGISTRY.read_text()
        self.cm.tmux = lambda *args, **kwargs: self.fail("must refuse before touching tmux")
        with contextlib.redirect_stderr(io.StringIO()) as err, self.assertRaises(SystemExit):
            self.cm.main(["rebuild", "work"])
        self.assertIn("worker records but no resume_state", err.getvalue())
        self.assertEqual(self.cm.REGISTRY.read_text(), before)

    def test_explicit_unlabelled_id_correction_keeps_registry_only_settings(self):
        self.rows[1]["managed_label"] = ""
        self.rows[1]["agent_args"] = ""
        self.cm.main(["reg", "worker", "add", "work", SID, "harness=pi", "model=worker-model",
                      "effort=high", 'agent_args=["--session-control"]', "custom=keep me"])
        self.cm.main(["identify", "work", "--pane", "%2", "--session-id", "corrected"])
        worker = self.workers()[0]
        self.assertEqual(worker["session_id"], "corrected")
        self.assertEqual(worker["effort"], "high")
        self.assertEqual(worker["model"], "worker-model")
        self.assertEqual(json.loads(worker["agent_args"]), ["--session-control"])
        self.assertEqual(worker["custom"], "keep me")

    def test_identity_correction_rejects_duplicate_before_mutating(self):
        self.rows[1].update(managed_harness="claude")
        self.rows[0].update(managed_id=SID)
        self.cm.main(["reg", "set", "work", f"resumed_session_id={SID}"])
        before = self.cm.REGISTRY.read_text()
        self.cm.tmux = lambda *args, **kwargs: self.fail("must refuse before editing pane identity")
        with contextlib.redirect_stderr(io.StringIO()) as err, self.assertRaises(SystemExit):
            self.cm.main(["identify", "work", "--pane", "%2", "--session-id", SID])
        self.assertIn("already bound", err.getvalue())
        self.assertEqual(self.cm.REGISTRY.read_text(), before)

    def test_native_switch_keeps_settings_by_label_without_guessing_identity(self):
        self.cm.main(["reg", "worker", "add", "work", "old", "harness=codex", "label=review", "effort=high", "custom=keep me"])
        self.rows[1].update(managed_harness="codex", managed_id="", title=SID)
        self.cm.record_pane_ids("work")
        worker = self.workers()[0]
        self.assertEqual(worker["session_id"], SID)
        self.assertEqual(worker["effort"], "high")
        self.assertEqual(worker["custom"], "keep me")
        self.rows[1]["title"] = "unknown"
        self.assertIsNone(self.cm.inspect_panes("work")[1]["session_id"])

    def test_unknown_native_id_keeps_bound_recovery_metadata(self):
        old = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
        self.cm.main(["reg", "worker", "add", "work", old, "harness=codex", "label=review",
                      "effort=high", 'agent_args=["--safe-flag"]', "custom=keep me"])
        self.rows[1].update(managed_harness="codex", managed_id=old, managed_label="", agent_args="",
                            title=SID[:29] + "...")
        self.cm.codex_ids = lambda cwd, prefix: set()
        before = self.workers()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as result:
            self.cm.main(["identify", "work"])
        self.assertEqual(result.exception.code, 3)
        self.assertEqual(self.workers(), before)
        self.assertIsNone(self.cm.resolve_target("work-review")["session_id"])
        path, missing, _ = self.cm.write_resume_state("work", Path(self.tmp.name) / "resume.md")
        saved = self.cm.parse_resume_state(path.read_text())["windows"][0]["panes"][1]
        self.assertTrue(missing)
        self.assertEqual((saved["label"], saved["effort"], saved["command"]), ("review", "high", ""))
        self.assertEqual(json.loads(saved["agent_args"]), ["--safe-flag"])
        with contextlib.redirect_stdout(io.StringIO()) as out:
            self.cm.main(["reconcile"])
        self.assertIn("recovery-only", out.getvalue())
        self.assertNotIn("stale-worker", out.getvalue())

    def test_secondary_hook_does_not_invent_unregistered_parent_harness(self):
        self.cm.REGISTRY.write_text("# Sessions\n")
        self.fire("%2", "pi", "SessionStart", SID)
        self.assertEqual(self.cm.Registry.load().entries(), [])
        self.assertEqual(self.rows[1]["managed_harness"], "pi")

    def test_command_wrapper_hook_does_not_turn_it_into_a_chat_agent(self):
        self.rows[1]["managed_harness"] = "command"
        self.fire("%2", "pi", "SessionStart", SID)
        self.assertEqual(self.workers(), [])
        self.assertEqual(self.cm.resolve_target("work-review")["harness"], "command")

    def test_primary_auto_end_keeps_other_panes(self):
        self.cm.main(["reg", "set", "work", "auto=true"])
        self.fire("%1", "claude", "SessionEnd", "lead", reason="exit")
        entry = self.cm.Registry.load().entry("work")
        self.assertIsNotNone(entry)
        self.assertIsNone(entry.get("auto"))

    def test_resolver_and_whoami_use_own_harness(self):
        self.cm.record_pane_ids("work")
        resolved = self.cm.resolve_target("work-review")
        self.assertEqual((resolved["pane_id"], resolved["harness"], resolved["entry_harness"]), ("%2", "pi", "claude"))
        self.assertEqual(resolved["agent_args"], [])
        with patch.dict(os.environ, {"TMUX_PANE": "%2"}), contextlib.redirect_stdout(io.StringIO()) as out:
            self.cm.main(["whoami", "--json"])
        info = json.loads(out.getvalue())
        self.assertEqual((info["pane"], info["session"], info["entry"]), ("%2", "work", "work"))
        self.assertEqual(info["name"], "work-review")
        self.assertEqual(info["harness"], "pi")
        self.assertFalse(info["primary"])

    def test_resolver_rejects_global_name_collision(self):
        self.cm.main(["reg", "new", "work-review", "harness=codex", "tmux_session=other"])
        self.rows.append(self.row("%3", "codex", SID, primary=True, session="other"))
        with self.assertRaisesRegex(self.cm.CmError, "ambiguous target"):
            self.cm.resolve_target("work-review")
        self.assertEqual(self.cm.resolve_target("%2")["harness"], "pi")
        self.assertEqual(self.cm.resolve_target("%3")["harness"], "codex")

    def test_untracked_pane_requires_explicit_address(self):
        self.rows.append(self.row("%3", "pi", SID, session="untracked"))
        self.assertEqual(self.cm.resolve_target("%3")["name"], "%3")
        with self.assertRaisesRegex(self.cm.CmError, "no live pane"):
            self.cm.resolve_target("untracked")

    def test_secondary_cannot_shutdown_or_wrap_container_without_opt_in(self):
        before = self.cm.REGISTRY.read_text()
        with patch.object(self.cm, "my_pane_quiet", return_value="%2"):
            for command in ("shutdown", "wrap"):
                with contextlib.redirect_stderr(io.StringIO()) as err, self.assertRaises(SystemExit):
                    self.cm.main([command, "work"])
                self.assertIn("--whole-session", err.getvalue())
                self.assertEqual(self.cm.REGISTRY.read_text(), before)
            self.cm.guard_secondary(self.cm.Registry.load().entry("work"), whole_session=True)
        for source in ("%1", "%99", None):
            with patch.object(self.cm, "my_pane_quiet", return_value=source):
                self.cm.guard_secondary(self.cm.Registry.load().entry("work"))

    def test_secondary_is_not_promoted_when_primary_pane_disappears(self):
        self.rows.pop(0)
        self.rows[0]["pane"] = 0
        self.assertFalse(self.cm.inspect_panes("work")[0]["primary"])
        with patch.object(self.cm, "my_pane_quiet", return_value="%2"), self.assertRaises(self.cm.CmError):
            self.cm.guard_secondary(self.cm.Registry.load().entry("work"))

    def test_reconcile_accepts_mixed_but_reports_real_conflict(self):
        self.cm.record_pane_ids("work")
        with contextlib.redirect_stdout(io.StringIO()) as out:
            self.cm.main(["reconcile"])
        self.assertEqual(out.getvalue().strip(), "ok")
        self.cm.ps_table = lambda: [(12, 1, "t", "codex")]
        with contextlib.redirect_stdout(io.StringIO()) as out:
            self.cm.main(["reconcile"])
        self.assertIn("harness-conflict", out.getvalue())
        with self.assertRaisesRegex(self.cm.CmError, "conflicts"):
            self.cm.record_pane_ids("work")

    def test_nested_agent_tool_is_not_a_primary_harness_conflict(self):
        self.cm.ps_table = lambda: [(11, 1, "t", "2.1.284"), (99, 11, "t", "pi --print task")]
        pane = self.cm.inspect_panes("work")[0]
        self.assertEqual(pane["kind"], "claude")
        self.assertFalse(pane["harness_conflict"])
        self.assertEqual(pane["session_id"], "lead")

    def test_log_tracks_harness_scoped_ids(self):
        self.cm.main(["reg", "worker", "add", "work", SID, "harness=claude"])
        for harness in ("claude", "pi"):
            self.cm.append_log(dict(event="end", harness=harness, session_id=SID, tmux_session="gone"))
        with contextlib.redirect_stdout(io.StringIO()) as out:
            self.cm.main(["log", "--untracked"])
        self.assertIn("\tpi\t", out.getvalue())
        self.assertNotIn("\tclaude\t", out.getvalue())


class MixedTmuxTests(IsolatedTmuxCase):
    def setUp(self):
        super().setUp()
        self.cm.INPUT_BOX = self.root / "no-input-box"  # never inspect the real tmux server
        observe = self.cm.wait_for_start
        self.cm.wait_for_start = lambda pane, timeout: observe(pane, timeout, settle=0.1)
        self.cm.my_pane_quiet = lambda: None  # test runner is outside this isolated server
        for harness in self.cm.AGENTS:
            script = self.bin / harness
            title = ("if [ \"$1\" = resume ]; then sid=$2; else sid=$(printf '11111111-2222-3333-4444-%012d' $$); fi\n"
                     "printf '\\033]0;%s\\007' \"$sid\"\n") if harness == "codex" else ""
            script.write_text("#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$AMX_STATE_DIR/argv-$TMUX_PANE\"\n" + title + "exec sleep 120\n")
            script.chmod(0o755)
        self.sessions = self.root / "sessions"
        env = patch.dict(os.environ, {"PI_CODING_AGENT_SESSION_DIR": str(self.sessions)})
        env.start()
        self.addCleanup(env.stop)

    def save_pi_transcripts(self, session):
        for pane in self.cm.inspect_panes(session):
            if pane["kind"] == "pi":
                directory = self.sessions / self.cm.pi_slug(pane["cwd"])
                directory.mkdir(parents=True, exist_ok=True)
                (directory / f"{pane['session_id']}.jsonl").write_text(json.dumps(dict(id=pane["session_id"], cwd=pane["cwd"])) + "\n")

    def test_all_agent_pairs_and_commands_stay_in_requested_container(self):
        for lead in self.cm.AGENTS:
            with self.subTest(lead=lead):
                self.cm.main(["spawn", "--harness", lead, "--id", lead, "--cwd", str(self.root)])
                for worker in self.cm.AGENTS:
                    self.cm.main(["spawn", "--into", lead, "--harness", worker, "--label", worker,
                                  "--window", worker])
                self.cm.main(["spawn", "--into", lead, "--label", "server", "--command", "sleep 120", "--window", "server"])
                for _ in range(30):
                    panes = self.cm.inspect_panes(lead)
                    if all(p["session_id"] for p in panes if p["kind"] in self.cm.AGENTS):
                        break
                    time.sleep(0.1)
                self.cm.record_pane_ids(lead)  # running need not mean native identity was ready
                self.assertEqual([p["kind"] for p in panes], [lead, "claude", "pi", "codex", "cmd"])
                self.assertEqual([w["harness"] for w in self.cm.worker_records(self.cm.Registry.load().entry(lead))],
                                 ["claude", "pi", "codex"])
                self.assertTrue(all(p["session"] == lead for p in panes))
                self.assertEqual(sum(p["primary"] for p in panes), 1)
                with contextlib.redirect_stdout(io.StringIO()) as out:
                    self.cm.main(["reconcile"])
                self.assertEqual(out.getvalue().strip(), "ok")
        self.assertEqual(set(self.cm.list_sessions()), {"anchor", "claude", "pi", "codex"})

    def test_mixed_snapshot_rebuild_keeps_ids_settings_roles_and_commands(self):
        self.spawn("--model", "lead-model")
        self.cm.main(["spawn", "--into", "work", "--harness", "claude", "--label", "review",
                      "--effort", "high", "--model", "review-model", "--agent-arg=--safe-flag"])
        self.cm.main(["spawn", "--into", "work", "--harness", "codex", "--label", "code", "--window", "code", "--effort", "low"])
        self.cm.main(["spawn", "--into", "work", "--label", "server", "--window", "server", "--command", "sleep 120"])
        before = self.cm.inspect_panes("work")
        self.save_pi_transcripts("work")
        self.cm.main(["shutdown", "work"])
        entry = self.cm.Registry.load().entry("work")
        saved = self.cm.parse_resume_state(Path(entry.get("resume_state")).read_text())
        review = saved["windows"][0]["panes"][1]
        self.assertEqual(review["effort"], "high")
        self.assertIn("--effort high", review["command"])
        self.assertIn("--safe-flag", review["command"])
        self.assertEqual(self.cm.entry_harnesses(entry), {"pi", "claude", "codex", "command"})
        self.cm.main(["rebuild", "work"])
        after = self.cm.inspect_panes("work")
        for key in ("kind", "session_id", "label", "effort", "model", "primary", "agent_args", "window_name"):
            self.assertEqual([p.get(key) for p in before], [p.get(key) for p in after], key)
        argv = (self.cm.STATE / f"argv-{after[1]['pane_id']}").read_text().splitlines()
        self.assertIn("high", argv)
        self.assertIn("--safe-flag", argv)
        self.assertEqual(self.cm.resolve_target("work-review")["harness"], "claude")
        self.assertEqual(self.cm.resolve_target("work-server")["harness"], "command")
        self.assertTrue(all(not p["dead"] for p in after))

    def test_generic_parent_can_host_recognized_agent(self):
        self.cm.main(["spawn", "--id", "generic", "--cwd", str(self.root), "--command", "sleep 120"])
        self.cm.main(["spawn", "--into", "generic", "--harness", "pi", "--label", "agent"])
        self.save_pi_transcripts("generic")
        self.cm.main(["shutdown", "generic"])
        self.cm.main(["rebuild", "generic"])
        resolved = self.cm.resolve_target("generic-agent")
        self.assertEqual((resolved["harness"], resolved["entry_harness"], resolved["primary"]), ("pi", "command", False))

    def test_identity_and_shutdown_survive_duplicate_native_ids_across_harnesses(self):
        self.cm.mint_id = lambda: SID
        self.spawn()
        self.cm.main(["spawn", "--into", "work", "--harness", "claude", "--label", "review"])
        self.cm.main(["identify", "work"])
        self.assertEqual(self.cm.entry_keys(self.cm.Registry.load().entry("work")), [("pi", SID), ("claude", SID)])
        self.save_pi_transcripts("work")
        self.cm.main(["shutdown", "work"])
        self.cm.main(["rebuild", "work"])
        self.assertEqual([p["session_id"] for p in self.cm.inspect_panes("work")], [SID, SID])

    def test_explicit_split_cannot_escape_requested_session(self):
        self.spawn()
        anchor = self.cm.pane_rows("anchor")[0]["pane_id"]
        before = len(self.cm.pane_rows())
        with contextlib.redirect_stderr(io.StringIO()) as err, self.assertRaises(SystemExit):
            self.cm.main(["spawn", "--into", "work", "--harness", "claude", "--label", "review", "--split", anchor])
        self.assertIn("not in requested session", err.getvalue())
        self.assertEqual(len(self.cm.pane_rows()), before)

    def test_launch_settings_fall_back_to_registry_without_pane_options(self):
        self.spawn()
        self.cm.main(["spawn", "--into", "work", "--harness", "claude", "--label", "review",
                      "--effort", "high", "--agent-arg=--safe-flag"])
        pane = self.cm.resolve_target("work-review")["pane_id"]
        for key in ("effort", "agent_args", "label"):
            self.tmux("set-option", "-pu", "-t", pane, f"@amx_{key}")
        resolved = self.cm.resolve_target("work-review")
        self.assertEqual(resolved["effort"], "high")
        self.assertEqual(resolved["agent_args"], ["--safe-flag"])

    def test_unknown_agent_returns_as_labelled_shell_not_chat_target(self):
        (self.bin / "codex").write_text(f"#!/bin/sh\nprintf '\\033]0;{SID[:29]}...\\007'\nexec sleep 120\n")
        self.cm.codex_ids = lambda cwd, prefix: set()
        self.spawn()
        self.cm.main(["spawn", "--into", "work", "--harness", "codex", "--label", "code", "--effort", "low"])
        old = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
        self.cm.main(["reg", "worker", "add", "work", old, "harness=codex", "label=code", "custom=keep me"])
        pane = self.cm.resolve_target("work-code")["pane_id"]
        self.tmux("set-option", "-p", "-t", pane, "@amx_session_id", old)
        self.tmux("select-pane", "-t", pane, "-T", SID[:29] + "...")
        self.save_pi_transcripts("work")
        self.cm.main(["shutdown", "work", "--force"])
        entry = self.cm.Registry.load().entry("work")
        saved = self.cm.parse_resume_state(Path(entry.get("resume_state")).read_text())["windows"][0]["panes"][1]
        self.assertEqual((saved["harness"], saved["label"], saved["effort"], saved["command"]), ("codex", "code", "low", ""))
        self.cm.main(["rebuild", "work"])
        resolved = self.cm.resolve_target("work-code")
        self.assertEqual(resolved["harness"], "command")
        self.assertIsNone(resolved["session_id"])
        self.assertFalse(resolved["primary"])
        worker = self.cm.worker_records(self.cm.Registry.load().entry("work"))[0]
        self.assertEqual((worker["session_id"], worker["custom"]), (old, "keep me"))
        self.cm.main(["identify", "work"])
        self.assertEqual(self.cm.worker_records(self.cm.Registry.load().entry("work"))[0], worker)

    def test_lost_primary_does_not_promote_worker_on_rebuild(self):
        self.spawn()
        self.cm.main(["spawn", "--into", "work", "--harness", "claude", "--label", "review"])
        self.tmux("kill-pane", "-t", self.cm.inspect_panes("work")[0]["pane_id"])
        self.cm.main(["shutdown", "work"])
        self.cm.main(["rebuild", "work"])
        pane = self.cm.inspect_panes("work")[0]
        self.assertFalse(pane["primary"])
        self.assertEqual(pane["name"], "work-review")
        self.assertEqual(self.cm.Registry.load().entry("work").get("harness"), "pi")
