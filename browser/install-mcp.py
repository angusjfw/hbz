#!/usr/bin/env python3
"""Register the browser MCP servers in Claude Code and pi, and fetch Playwright's browser.

chrome-devtools-mcp attaches to your own Chrome. Its new_page opens a tab in
whichever window you're using, so it's installed pinned and patched to open
each agent page in a window of its own.

Both agents keep their MCP server lists machine-local (they also hold work
endpoints), so this upserts just the entries in mcp-servers.json.
"""
import json
import subprocess
import sys
from pathlib import Path

HOME = str(Path.home())
DEVTOOLS_VERSION = "1.10.1"
DEVTOOLS_PREFIX = Path(HOME, ".local/share/chrome-devtools-mcp")
servers = json.loads(
    (Path(__file__).parent / "mcp-servers.json").read_text().replace("__HOME__", HOME)
)

subprocess.run(
    ["npm", "install", "--prefix", str(DEVTOOLS_PREFIX), "--no-save", "--no-fund", "--no-audit",
     f"chrome-devtools-mcp@{DEVTOOLS_VERSION}"],
    check=True,
)
context = DEVTOOLS_PREFIX / "node_modules/chrome-devtools-mcp/build/src/McpContext.js"
source = context.read_text()
stock = "page = await this.browser.newPage({ background });"
patched = "page = await this.browser.newPage({ background, type: 'window' });"
if patched not in source:
    if source.count(stock) != 1:
        sys.exit(f"chrome-devtools-mcp {DEVTOOLS_VERSION}: new_page code changed, update the patch in {__file__}")
    context.write_text(source.replace(stock, patched))

for name, spec in servers.items():
    subprocess.run(["claude", "mcp", "remove", "-s", "user", name], capture_output=True)
    subprocess.run(["claude", "mcp", "add-json", "-s", "user", name, json.dumps(spec)], check=True)

pi_config = Path(HOME, ".pi/agent/mcp-adapter.json")
if pi_config.parent.is_dir():
    config = json.loads(pi_config.read_text()) if pi_config.exists() else {}
    config.setdefault("mcpServers", {}).update(servers)
    pi_config.write_text(json.dumps(config, indent=2) + "\n")
    print(f"updated {pi_config}")

# Playwright looks for a browser build tied to its own version, so install the pinned one's
package = next(a for a in servers["playwright"]["args"] if a.startswith("@playwright/mcp@"))
subprocess.run(["npx", "-y", package, "install-browser", "chrome-for-testing"], check=True)
