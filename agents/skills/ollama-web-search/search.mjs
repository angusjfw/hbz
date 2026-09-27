#!/usr/bin/env node

import { existsSync, readFileSync } from "fs";
import { homedir } from "os";
import { join } from "path";

const API_URL = "https://ollama.com/api/web_search";

function parseArgs(argv) {
	const out = {
		maxResults: 5,
		json: false,
		help: false,
		query: "",
	};
	const positional = [];
	for (let i = 0; i < argv.length; i++) {
		const arg = argv[i];
		if (arg === "--help" || arg === "-h") {
			out.help = true;
			continue;
		}
		if (arg === "--json") {
			out.json = true;
			continue;
		}
		if (arg === "--max-results") {
			const n = Number(argv[++i]);
			out.maxResults = Number.isFinite(n) && n > 0 ? Math.min(n, 10) : 5;
			continue;
		}
		if (arg.startsWith("--max-results=")) {
			const n = Number(arg.slice("--max-results=".length));
			out.maxResults = Number.isFinite(n) && n > 0 ? Math.min(n, 10) : 5;
			continue;
		}
		positional.push(arg);
	}
	out.query = positional.join(" ").trim();
	return out;
}

function usage() {
	return `Usage:
  node search.mjs "<query>" [--max-results N] [--json]

Examples:
  node search.mjs "latest python release"
  node search.mjs "vite 7 breaking changes" --max-results 10 --json`;
}

function readJson(path, fallback = {}) {
	if (!existsSync(path)) return fallback;
	try {
		return JSON.parse(readFileSync(path, "utf8"));
	} catch {
		return fallback;
	}
}

function getAgentDir() {
	const configured = process.env.PI_CODING_AGENT_DIR;
	if (!configured) return join(homedir(), ".pi", "agent");
	if (configured === "~") return homedir();
	if (configured.startsWith("~/")) return join(homedir(), configured.slice(2));
	return configured;
}

function resolveApiKey() {
	if (process.env.OLLAMA_API_KEY) return process.env.OLLAMA_API_KEY;
	const auth = readJson(join(getAgentDir(), "auth.json"), {});
	const entry = auth?.ollama;
	if (!entry) return undefined;
	if (entry.type === "api_key" && entry.key) return entry.key;
	if (entry.key) return entry.key;
	return undefined;
}

async function search({ query, maxResults }) {
	const apiKey = resolveApiKey();
	if (!apiKey) {
		throw new Error(
			"OLLAMA_API_KEY not set. Add it to ~/.pi/agent/auth.json under 'ollama' or export it.",
		);
	}

	const res = await fetch(API_URL, {
		method: "POST",
		headers: {
			authorization: `Bearer ${apiKey}`,
			"content-type": "application/json",
		},
		body: JSON.stringify({ query, max_results: maxResults }),
	});

	const text = await res.text();
	let parsed;
	try {
		parsed = JSON.parse(text);
	} catch {
		parsed = { raw: text };
	}

	if (!res.ok) {
		throw new Error(
			`Ollama web search failed (${res.status}): ${parsed?.error || text || "unknown error"}`,
		);
	}

	if (!Array.isArray(parsed.results)) {
		throw new Error("Unexpected response shape from Ollama web search");
	}
	return parsed.results;
}

function printResults(results) {
	for (const r of results) {
		console.log(`\n${r.title}`);
		console.log(r.url);
		if (r.content) console.log(r.content);
	}
}

async function main() {
	const args = parseArgs(process.argv.slice(2));
	if (args.help || !args.query) {
		console.error(usage());
		process.exit(args.help ? 0 : 1);
	}

	const results = await search(args);
	if (args.json) {
		console.log(JSON.stringify({ query: args.query, results }, null, 2));
		return;
	}

	console.log(`Results for: ${args.query}`);
	printResults(results);
}

main().catch((err) => {
	console.error(`Error: ${err?.message || err}`);
	process.exit(1);
});
