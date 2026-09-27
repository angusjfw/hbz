#!/usr/bin/env node

import { homedir } from "node:os";
import { join } from "node:path";
import { findBrowserProcesses } from "./browser-processes.js";

const args = process.argv.slice(2);
if (args.some((arg) => arg !== "--all")) {
  console.error("Usage: stop.js [--all]");
  console.error("Stops the skill browser on BROWSER_DEBUG_PORT (default 9222).");
  console.error("--all stops skill browsers on every port, including headed ones.");
  process.exit(1);
}
const port = Number(process.env.BROWSER_DEBUG_PORT || 9222);
if (!Number.isInteger(port) || port < 1 || port > 65535) {
  console.error("✗ Invalid BROWSER_DEBUG_PORT (expected 1-65535)");
  process.exit(1);
}
const root = join(process.env.HOME || homedir(), ".cache", "agent-web", "browser");
const dirs = [join(root, "fresh-profile"), join(root, "profile-copy")];
const browsers = findBrowserProcesses(dirs).filter(
  (browser) => args.includes("--all") || browser.port === port,
);
if (!browsers.length) {
  console.log("✓ No matching skill browser is running");
  process.exit(0);
}

for (const browser of browsers) {
  // Revalidate ownership immediately before signaling; never trust state.json.
  if (!findBrowserProcesses(dirs).some((current) =>
    current.pid === browser.pid && current.userDataDir === browser.userDataDir &&
    current.port === browser.port)) continue;
  try {
    process.kill(browser.pid, "SIGTERM");
  } catch (error) {
    if (error.code !== "ESRCH") throw error;
  }
}

for (let i = 0; i < 40; i++) {
  const remaining = findBrowserProcesses(dirs).filter((current) =>
    browsers.some((browser) => browser.pid === current.pid));
  if (!remaining.length) {
    console.log("✓ Skill browser stopped; cached profiles preserved");
    process.exit(0);
  }
  await new Promise((resolve) => setTimeout(resolve, 250));
}
console.error("✗ Browser has not exited yet. Not forcing shutdown to avoid profile corruption.");
process.exit(1);
