#!/usr/bin/env python3
"""Writes one file that opens from disk or can be put anywhere a single page can go.

tools/bundle.py            build/shootout.html: the C game, its page, host and .wasm
tools/bundle.py engine     build/engine.html: the Rust engine, its page and controls, and web/default.pack
"""
import base64
import sys

game = sys.argv[1] if len(sys.argv) > 1 else "shootout"
b64 = lambda path: base64.b64encode(open(path, "rb").read()).decode()
host = open("web/wtk.js").read().replace("export async function boot", "async function boot")
page = open("web/engine.html" if game == "engine" else "web/index.html").read()
start = page.index('<script type="module">')
end = page.index("</script>", start)
if game == "engine":
    # The bytes are unpacked in the page, with nothing fetched, so it works where fetching is not allowed.
    rest = page[page.index("const buttons =", start):end]
    script = (f'<script type="module">\n{host}\n'
              'const bytes = (b64) => Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));\n'
              f'const game = await boot(document.getElementById("screen"), bytes("{b64("web/engine.wasm")}"), undefined, bytes("{b64("web/default.pack")}"));\n  {rest}')
else:
    script = f'<script type="module">\n{host}\nboot(document.getElementById("screen"), "data:application/wasm;base64,{b64(f"web/{game}.wasm")}");\n'
open(f"build/{game}.html", "w").write(page[:start] + script + page[end:])
print(f"build/{game}.html")
