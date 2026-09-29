import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("merge-settings.py")


class MergeSettings(unittest.TestCase):
    def setUp(self):
        self.dir = Path(tempfile.mkdtemp())
        self.baseline = self.dir / "settings.json.example"
        self.live = self.dir / "settings.json"
        self.applied = self.dir / "settings.json.applied"

    def run_merge(self, baseline, live=None, applied=None, *extra):
        self.baseline.write_text(json.dumps(baseline))
        if live is not None:
            self.live.write_text(json.dumps(live))
        if applied is not None:
            self.applied.write_text(json.dumps(applied))
        r = subprocess.run([sys.executable, SCRIPT, self.baseline, self.live, *extra],
                           capture_output=True, text=True, check=True)
        return json.loads(self.live.read_text()), r.stderr

    def test_no_live_file_takes_baseline(self):
        out, _ = self.run_merge({"model": "a", "list": [1]})
        self.assertEqual(out, {"model": "a", "list": [1]})
        self.assertEqual(json.loads(self.applied.read_text()), {"model": "a", "list": [1]})

    def test_untouched_keys_follow_baseline_changes(self):
        out, err = self.run_merge({"list": [1, 2], "gone": None, "new": 1},
                                  live={"list": [1], "gone": 0, "mine": 1},
                                  applied={"list": [1], "gone": 0})
        self.assertEqual(out, {"list": [1, 2], "gone": None, "mine": 1, "new": 1})
        self.assertEqual(err, "")

    def test_baseline_deletion_applies_when_untouched(self):
        out, _ = self.run_merge({}, live={"old": 1}, applied={"old": 1})
        self.assertEqual(out, {})

    def test_local_change_survives_and_is_reported(self):
        out, err = self.run_merge({"model": "kimi", "list": [1]},
                                  live={"model": "sol", "list": [1, 9]},
                                  applied={"model": "kimi", "list": [1]})
        self.assertEqual(out, {"model": "sol", "list": [1, 9]})
        self.assertIn("local override model", err)
        self.assertIn('local override list: local list +[9] vs baseline', err)

    def test_local_deletion_survives(self):
        out, _ = self.run_merge({"a": 1}, live={}, applied={"a": 1})
        self.assertEqual(out, {})

    def test_both_changed_keeps_local_and_warns(self):
        out, err = self.run_merge({"list": [1, 2]}, live={"list": [1, 9]}, applied={"list": [1]})
        self.assertEqual(out, {"list": [1, 9]})
        self.assertIn("WARNING both changed list", err)

    def test_objects_merge_per_key(self):
        out, _ = self.run_merge({"env": {"A": "2", "B": "1"}},
                                live={"env": {"A": "1", "C": "x"}},
                                applied={"env": {"A": "1"}})
        self.assertEqual(out, {"env": {"A": "2", "C": "x", "B": "1"}})

    def test_first_run_treats_differences_as_local(self):
        out, err = self.run_merge({"model": "kimi", "list": [1], "new": 1},
                                  live={"model": "sol", "list": [1], "tool": 1})
        self.assertEqual(out, {"model": "sol", "list": [1], "tool": 1, "new": 1})
        self.assertIn("local override model", err)
        self.assertNotIn("tool", err)

    def test_target_regular_file_seeds_missing_live(self):
        target = self.dir / "home-settings.json"
        target.write_text(json.dumps({"model": "sol"}))
        out, err = self.run_merge({"model": "kimi", "x": 1}, None, None, "--target", str(target))
        self.assertEqual(out, {"model": "sol", "x": 1})
        self.assertFalse(target.exists())
        self.assertTrue((self.dir / "home-settings.json.pre-hbz").exists())
        self.assertIn("seeded", err)

    def test_target_symlink_left_alone(self):
        target = self.dir / "link.json"
        self.live.write_text("{}")
        target.symlink_to(self.live)
        self.run_merge({}, None, None, "--target", str(target))
        self.assertTrue(target.is_symlink())


if __name__ == "__main__":
    unittest.main()
