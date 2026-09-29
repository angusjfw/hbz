#!/usr/bin/env python3
"""Three-way merge of a committed settings baseline into a live settings file.

    merge-settings.py BASELINE LIVE [--target PATH]

The live file is gitignored and written to by the tool itself (and by hand),
so neither side can simply win. The baseline applied on the previous run is
kept beside the live file as LIVE.applied and used as the common ancestor:

- a key unchanged locally since the last run takes the new baseline value
  (including deletion);
- a key changed locally keeps its local value;
- a key changed on both sides keeps the local value and prints a warning.

Objects merge key by key; arrays and scalars are whole values. With no
.applied file yet (first run), any value that differs from the baseline is
treated as a local change.

--target is the path the live file gets symlinked to. If it is a regular
file, it is moved aside to TARGET.pre-hbz; when LIVE doesn't exist yet, its
contents seed LIVE, so settings made before the first install aren't lost.
"""
import argparse
import json
import os
import shutil
import sys
from pathlib import Path

MISSING = object()


def load(path):
    text = path.read_text() if path.exists() else ""
    return json.loads(text) if text.strip() else {}


def show(value):
    return "(absent)" if value is MISSING else json.dumps(value)


def describe(local, base):
    if isinstance(local, list) and isinstance(base, list):
        added = [x for x in local if x not in base]
        removed = [x for x in base if x not in local]
        if added or removed:
            parts = [f"+{json.dumps(added)}" if added else "", f"-{json.dumps(removed)}" if removed else ""]
            return "local list " + " ".join(p for p in parts if p) + " vs baseline"
    return f"kept {show(local)}; baseline has {show(base)}"


def merge(old, new, live, path, notes):
    """Return the merged object; append (kind, keypath, detail) to notes."""
    out = {}
    for key in list(live) + [k for k in new if k not in live]:
        o = old.get(key, MISSING) if isinstance(old, dict) else MISSING
        n = new.get(key, MISSING)
        l = live.get(key, MISSING)
        keypath = f"{path}.{key}" if path else key
        if isinstance(l, dict) and isinstance(n, dict) and (o is MISSING or isinstance(o, dict)):
            out[key] = merge(o if isinstance(o, dict) else {}, n, l, keypath, notes)
            continue
        detail = describe(l, n)
        if l == n:
            value = l
        elif old is None:  # first run: no ancestor, so any difference counts as local
            value = n if l is MISSING else l
            if l is not MISSING and n is not MISSING:
                notes.append(("local", keypath, detail))
        elif l == o:
            value = n
        elif n == o:
            value = l
            if n is not MISSING:
                notes.append(("local", keypath, detail))
        else:
            value = l
            notes.append(("conflict", keypath, detail))
        if value is not MISSING:
            out[key] = value
    return out


def write_json(path, data):
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")
    os.replace(tmp, path)


def adopt(target, live):
    if not target.is_file() or target.is_symlink():
        return
    aside = target.with_name(target.name + ".pre-hbz")
    if not live.exists():
        shutil.copyfile(target, live)
        print(f"settings: seeded {live} from existing {target}", file=sys.stderr)
    else:
        print(f"settings: {target} was a regular file, not merged; compare it with {live}",
              file=sys.stderr)
    os.replace(target, aside)
    print(f"settings: moved {target} to {aside}", file=sys.stderr)


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("baseline", type=Path)
    p.add_argument("live", type=Path)
    p.add_argument("--target", type=Path, help="symlink location; a regular file there is moved aside")
    a = p.parse_args()

    if a.target:
        adopt(a.target.expanduser(), a.live)
    applied = a.live.with_name(a.live.name + ".applied")
    new = load(a.baseline)
    old = load(applied) if applied.exists() else None
    notes = []
    merged = merge(old, new, load(a.live), "", notes)
    write_json(a.live, merged)
    write_json(applied, new)
    for kind, keypath, detail in notes:
        label = "WARNING both changed" if kind == "conflict" else "local override"
        print(f"{a.live.name}: {label} {keypath}: {detail}", file=sys.stderr)


if __name__ == "__main__":
    main()
