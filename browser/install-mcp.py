#!/usr/bin/env python3
"""Set up the browser MCP servers for Claude Code and pi, and fetch Playwright's browser.

playwright runs once as a launchd user agent on 127.0.0.1 and both agents
connect to it over HTTP, instead of each session starting its own copy. Each
HTTP client gets its own browser context (no --shared-browser-context), so
sessions don't see each other's pages. --allowed-hosts rejects a foreign Host,
which stops DNS rebinding. Playwright doesn't check Origin, but a page on
another site still can't use it: requests it could send without a CORS
preflight get 415, and the preflight gets no CORS headers.

chrome-devtools attaches to your own Chrome, and Chrome asks you to approve each
new connection, so it can't be shared. Claude Code doesn't load it by default: a
session that needs it starts with `--mcp-config=~/.claude/mcp/chrome-devtools.json`
(`amx spawn --mcp chrome-devtools`). pi starts it on first use. It's installed
pinned and patched to open each agent page in a window of its own.

Both agents keep their MCP server lists machine-local (they also hold work
endpoints), so this upserts just the entries in mcp-servers.json.
"""
import json
import os
import plistlib
import shutil
import subprocess
import sys
import time
from pathlib import Path

HOME = str(Path.home())
DEVTOOLS_VERSION = "1.10.1"
DEVTOOLS_PREFIX = Path(HOME, ".local/share/chrome-devtools-mcp")
OPTIONAL_DIR = Path(HOME, ".claude/mcp")
PLAYWRIGHT_LABEL = "io.hbz.playwright-mcp"
LAUNCHD_PATH = f"{HOME}/.local/bin:/opt/homebrew/bin:/usr/bin:/bin"
servers = json.loads(
    (Path(__file__).parent / "mcp-servers.json").read_text().replace("__HOME__", HOME)
)
devtools = servers["chrome-devtools"]
playwright = servers["playwright"]
playwright_url = f"http://127.0.0.1:{playwright['port']}/mcp"

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

# Playwright looks for a browser build tied to its own version, so install the pinned one's
package = next(a for a in playwright["args"] if a.startswith("@playwright/mcp@"))
subprocess.run(["npx", "-y", package, "install-browser", "chrome-for-testing"], check=True)


def launchctl(*args):
    return subprocess.run(["launchctl", *args], capture_output=True, text=True)


if sys.platform == "darwin":
    domain = f"gui/{os.getuid()}"
    plist = Path(HOME, "Library/LaunchAgents", f"{PLAYWRIGHT_LABEL}.plist")
    log = Path(HOME, "Library/Logs/mcp/playwright.log")
    log.parent.mkdir(parents=True, exist_ok=True)
    # launchd doesn't search PATH for the program itself
    command = shutil.which(playwright["command"], path=LAUNCHD_PATH) or sys.exit(
        f"{playwright['command']} not found on {LAUNCHD_PATH}")
    plist.write_bytes(plistlib.dumps({
        "Label": PLAYWRIGHT_LABEL,
        "ProgramArguments": [command, *playwright["args"]],
        "EnvironmentVariables": {"PATH": LAUNCHD_PATH},
        "WorkingDirectory": HOME,
        "RunAtLoad": True,
        "KeepAlive": True,
        "StandardOutPath": str(log),
        "StandardErrorPath": str(log),
    }))
    # bootstrap fails while a bootout of the same label is still finishing
    launchctl("bootout", f"{domain}/{PLAYWRIGHT_LABEL}")
    for _ in range(50):
        if launchctl("print", f"{domain}/{PLAYWRIGHT_LABEL}").returncode != 0:
            break
        time.sleep(0.2)
    r = launchctl("bootstrap", domain, str(plist))
    if r.returncode != 0:
        sys.exit(f"could not bootstrap {PLAYWRIGHT_LABEL}: {r.stderr.strip()}")
    print(f"playwright: launchd {PLAYWRIGHT_LABEL}, {playwright_url}")


def claude_mcp(*args, check=False):
    return subprocess.run(["claude", "mcp", *args], capture_output=True, text=True, check=check)


claude_mcp("remove", "-s", "user", "playwright")
claude_mcp("add-json", "-s", "user", "playwright", json.dumps({"type": "http", "url": playwright_url}), check=True)
claude_mcp("remove", "-s", "user", "chrome-devtools")
OPTIONAL_DIR.mkdir(parents=True, exist_ok=True)
optional = OPTIONAL_DIR / "chrome-devtools.json"
optional.write_text(json.dumps({"mcpServers": {"chrome-devtools": devtools}}, indent=2) + "\n")
print(f"chrome-devtools: opt-in for Claude Code via --mcp-config={optional}")

pi_config = Path(HOME, ".pi/agent/mcp-adapter.json")
if pi_config.parent.is_dir():
    config = json.loads(pi_config.read_text()) if pi_config.exists() else {}
    entries = config.setdefault("mcpServers", {})
    entries["playwright"] = {"url": playwright_url}
    # Chrome asks you to approve each new connection, so stay connected after first use
    # rather than dropping it on pi's idle timeout and prompting again
    entries["chrome-devtools"] = {**devtools, "lifecycle": "lazy-keep-alive"}
    pi_config.write_text(json.dumps(config, indent=2) + "\n")
    print(f"updated {pi_config}")
