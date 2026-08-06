#!/usr/bin/env python3
"""Classify the input box of a Claude Code pane.

    read-input-box.py <pane-target>   ->  <state>[\t<box text>]

States:

    no-box   no input box on screen (a dialog is up, or not a Claude pane)
    empty    box present, nothing in it
    ghost    box holds only Claude's own dim placeholder guess at what the
             user might type next — NOT anything the user wrote
    draft    box holds real, unsent user text — do not clobber it

The distinction is the SGR dim attribute (ESC[2m): the placeholder is
rendered dim, real draft text is not. `tmux capture-pane -p` strips
attributes, which makes the two indistinguishable — hence `-e` here.

Cursor column corroborates but cannot decide it: the placeholder always
leaves the cursor at the start of the box, and so does a real draft whose
author moved the cursor back to column 0 (vim NORMAL `0`). Dimness is
what separates them, so that is what this reads.
"""

import re
import subprocess
import sys

SGR = re.compile(r"\x1b\[([0-9;]*)m")
MARKER = "❯"  # ❯ prompt marker, always followed by U+00A0 in the box
NBSP = " "
RULE = "─"  # ─ box border


def tmux(*args):
    r = subprocess.run(("tmux",) + args, capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def plain(row):
    return SGR.sub("", row)


def is_rule(rows, i):
    """A border row: mostly ─. The top one may carry a branch/PR badge, so
    it is not required to be pure."""
    if not 0 <= i < len(rows):
        return False
    p = plain(rows[i]).rstrip()
    n = p.count(RULE)
    return n > 5 and n >= 0.5 * len(p)


def split_dim(row):
    """Return (all_text, text_outside_any_dim_run) for one row."""
    all_text, outside, dim, pos = [], [], False, 0
    for m in SGR.finditer(row):
        chunk = row[pos:m.start()]
        all_text.append(chunk)
        if not dim:
            outside.append(chunk)
        for code in m.group(1).split(";"):
            if code == "2":
                dim = True
            elif code in ("", "0", "22"):
                dim = False
        pos = m.end()
    tail = row[pos:]
    all_text.append(tail)
    if not dim:
        outside.append(tail)
    return "".join(all_text), "".join(outside)


def main():
    if len(sys.argv) != 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    target = sys.argv[1]

    height = tmux("display-message", "-p", "-t", target, "#{pane_height}")
    if not height or not height.strip().isdigit():
        print("no-box")
        return 0
    h = int(height.strip())

    # Visible screen only, with SGR escapes. No -J: joining wrapped lines
    # would break the 1:1 mapping between output lines and screen rows.
    screen = tmux("capture-pane", "-p", "-e", "-t", target, "-S", "0", "-E", str(h - 1))
    if not screen:
        print("no-box")
        return 0
    rows = screen.split("\n")

    # The box's prompt marker is ❯ followed by U+00A0. Selection lists
    # ("❯ 1. Yes") reuse the glyph with a normal space, so the NBSP is what
    # tells the input box apart from a dialog.
    start = None
    for i in range(len(rows) - 1, -1, -1):
        if not plain(rows[i]).lstrip().startswith(MARKER + NBSP):
            continue
        if not (is_rule(rows, i - 1) or is_rule(rows, i + 1)):
            continue  # a bare marker somewhere in the transcript
        start = i
        break
    if start is None:
        print("no-box")
        return 0

    # Content: the marker row with the marker stripped, plus any rows below
    # it up to the closing border, so wrapped multi-line drafts are covered.
    first = re.sub(r"^((?:\x1b\[[0-9;]*m)*)\s*" + MARKER + NBSP, r"\1", rows[start])
    content = [first]
    i = start + 1
    while i < len(rows) and not is_rule(rows, i):
        content.append(rows[i])
        i += 1

    all_text, outside = "", ""
    for row in content:
        a, o = split_dim(row)
        all_text += plain(a)
        outside += plain(o)
    all_text = all_text.strip().strip(NBSP).strip()
    outside = "".join(outside.split()).replace(NBSP, "")

    if not all_text:
        print("empty")
    elif not outside:
        print(f"ghost\t{all_text}")
    else:
        print(f"draft\t{all_text}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
