#!/usr/bin/env python3
"""Writes build/<game>.html: the page, the host and the .wasm in one file that opens from disk."""
import base64
import sys

game = sys.argv[1] if len(sys.argv) > 1 else "shootout"
wasm = base64.b64encode(open(f"web/{game}.wasm", "rb").read()).decode()
host = open("web/wtk.js").read().replace("export async function boot", "async function boot")
page = open("web/index.html").read()
start = page.index('<script type="module">')
end = page.index("</script>", start)
script = f'<script type="module">\n{host}\nboot(document.getElementById("screen"), "data:application/wasm;base64,{wasm}");\n'
open(f"build/{game}.html", "w").write(page[:start] + script + page[end:])
print(f"build/{game}.html")
