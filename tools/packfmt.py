#!/usr/bin/env python3
"""Writes a pack.json as the default pack keeps it: two spaces in, and a list of points (a shape,
or where bolts go) on one line, so a shape of twenty corners is one line and not sixty.

    tools/packfmt.py content/default/pack.json
"""
import json
import sys


def points(v):
    return isinstance(v, list) and v and all(isinstance(p, list) and len(p) == 2 and all(isinstance(n, (int, float)) for n in p) for p in v)


def dump(v, depth=0):
    pad, inner = "  " * depth, "  " * (depth + 1)
    if points(v):
        return json.dumps(v)
    if isinstance(v, dict):
        if not v:
            return "{}"
        return "{\n" + ",\n".join(f"{inner}{json.dumps(k, ensure_ascii=False)}: {dump(x, depth + 1)}" for k, x in v.items()) + "\n" + pad + "}"
    if isinstance(v, list):
        if not v:
            return "[]"
        return "[\n" + ",\n".join(inner + dump(x, depth + 1) for x in v) + "\n" + pad + "]"
    return json.dumps(v, ensure_ascii=False)


if __name__ == "__main__":
    for path in sys.argv[1:]:
        with open(path) as f:
            data = json.load(f)
        with open(path, "w") as f:
            f.write(dump(data) + "\n")
