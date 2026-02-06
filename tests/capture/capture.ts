#!/usr/bin/env bun
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

// --- Colors & constants ---
const BOLD = "\x1b[1m";
const RED = "\x1b[0;31m";
const GREEN = "\x1b[0;32m";
const YELLOW = "\x1b[0;33m";
const CYAN = "\x1b[0;36m";
const MAGENTA = "\x1b[0;35m";
const DIM = "\x1b[0;90m";
const NC = "\x1b[0m";

const KEYBOARD_TIMEOUT = Number(process.env.KEYBOARD_TIMEOUT ?? 30);
const THINKING_TIMEOUT = Number(process.env.THINKING_TIMEOUT ?? 300);
const DEFAULT_CLAUDE_ARGS = process.env.DEFAULT_CLAUDE_ARGS ?? "--model haiku";

const SCRIPT_DIR = path.dirname(new URL(import.meta.url).pathname);
const CAPTURE_DIR = SCRIPT_DIR;

// --- Semaphore for parallel execution ---

class Semaphore {
  private waiting: (() => void)[] = [];
  private active = 0;

  constructor(private readonly limit: number) {}

  async run<T>(fn: () => Promise<T>): Promise<T> {
    await this.acquire();
    try {
      return await fn();
    } finally {
      this.release();
    }
  }

  private acquire(): Promise<void> {
    if (this.active < this.limit) {
      this.active++;
      return Promise.resolve();
    }
    return new Promise((resolve) => this.waiting.push(resolve));
  }

  private release(): void {
    this.active--;
    const next = this.waiting.shift();
    if (next) {
      this.active++;
      next();
    }
  }
}

// --- Utility functions ---

function loadCaptureEnv(): void {
  const envFile = path.join(CAPTURE_DIR, ".env");
  if (fs.existsSync(envFile)) {
    const content = fs.readFileSync(envFile, "utf-8");
    for (const line of content.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith("#")) continue;
      const eqIdx = trimmed.indexOf("=");
      if (eqIdx === -1) continue;
      const key = trimmed.slice(0, eqIdx);
      const value = trimmed.slice(eqIdx + 1);
      process.env[key] = value;
    }
  }

  if (!process.env.CLAUDE_CODE_OAUTH_TOKEN) {
    process.stderr.write(
      `${RED}Error: CLAUDE_CODE_OAUTH_TOKEN not set${NC}\n` +
        "\n" +
        "Capture scripts require an OAuth token to authenticate with Claude CLI.\n" +
        "\n" +
        "Setup:\n" +
        "  1. Run: claude setup-token\n" +
        "  2. Add to tests/capture/.env:\n" +
        "     CLAUDE_CODE_OAUTH_TOKEN=<your-token>\n" +
        "\n" +
        "See: tests/capture/.env.example\n",
    );
    process.exit(1);
  }
}

function detectVersion(): string {
  const result = Bun.spawnSync(["claude", "--version"], {
    stdout: "pipe",
    stderr: "pipe",
  });
  const output = result.stdout.toString();
  const match = output.match(/(\d+\.\d+\.\d+)/);
  return match?.[1] ?? "";
}

function checkClaude(): void {
  const result = Bun.spawnSync(["which", "claude"], {
    stdout: "pipe",
    stderr: "pipe",
  });
  if (result.exitCode !== 0) {
    process.stderr.write(`${RED}Error: claude CLI not found in PATH${NC}\n`);
    process.exit(1);
  }
}

function checkCapsh(): void {
  const result = Bun.spawnSync(["which", "capsh"], {
    stdout: "pipe",
    stderr: "pipe",
  });
  if (result.exitCode !== 0) {
    process.stderr.write(
      `${RED}Error: capsh not found in PATH${NC}\n` +
        "Build with: cargo build --release -p capsh\n",
    );
    process.exit(1);
  }
}

// --- Script header parsing ---

function parseClaudeArgs(script: string): string {
  const content = fs.readFileSync(script, "utf-8");
  const match = content.match(/^# Args:\s*(.*)$/m);
  if (match) {
    const args = match[1].trim();
    if (args === "(none)" || args === "none") return "";
    return args;
  }
  return DEFAULT_CLAUDE_ARGS;
}

function parseWorkspace(script: string): string {
  const content = fs.readFileSync(script, "utf-8");
  const match = content.match(/^# Workspace:\s*(.*)$/m);
  if (match) {
    const ws = match[1].trim();
    if (ws === "(temp)" || ws === "temp") return "";
    return ws;
  }
  return "";
}

function parseConfigMode(script: string): "trusted" | "welcome-back" | "auth-only" | "empty" {
  const content = fs.readFileSync(script, "utf-8");
  const match = content.match(/^# Config:\s*(.*)$/m);
  if (match) {
    const mode = match[1].trim().split(/\s+/)[0];
    if (mode === "welcome-back" || mode === "auth-only" || mode === "empty") return mode;
  }
  return "trusted";
}

function parseEnv(script: string): Record<string, string> {
  const content = fs.readFileSync(script, "utf-8");
  const match = content.match(/^# Env:\s*(.*)$/m);
  if (!match) return {};
  const env: Record<string, string> = {};
  for (const pair of match[1].trim().split(/\s+/)) {
    const eqIdx = pair.indexOf("=");
    if (eqIdx === -1) continue;
    env[pair.slice(0, eqIdx)] = pair.slice(eqIdx + 1);
  }
  return env;
}

function parseTimeout(script: string): number | null {
  const content = fs.readFileSync(script, "utf-8");
  const match = content.match(/^# Timeout:\s*(\d+)/m);
  if (match) return Number(match[1]);
  return null;
}

// --- Config writing ---

function writeClaudeConfig(
  configDir: string,
  workspace: string,
  version: string,
): void {
  fs.mkdirSync(configDir, { recursive: true });
  const config = {
    hasCompletedOnboarding: true,
    lastOnboardingVersion: version,
    projects: {
      [workspace]: {
        hasTrustDialogAccepted: true,
        allowedTools: [],
      },
    },
  };
  fs.writeFileSync(
    path.join(configDir, ".claude.json"),
    JSON.stringify(config, null, 2) + "\n",
  );
}

function writeWelcomeBackConfig(
  configDir: string,
  workspace: string,
  version: string,
): void {
  fs.mkdirSync(configDir, { recursive: true });
  const config = {
    hasCompletedOnboarding: true,
    lastOnboardingVersion: version,
    projects: {
      [workspace]: {
        hasTrustDialogAccepted: true,
        allowedTools: [],
      },
    },
  };
  fs.writeFileSync(
    path.join(configDir, ".claude.json"),
    JSON.stringify(config, null, 2) + "\n",
  );
}

function writeAuthOnlyConfig(configDir: string, version: string): void {
  fs.mkdirSync(configDir, { recursive: true });
  const config = {
    hasCompletedOnboarding: true,
    lastOnboardingVersion: version,
    projects: {},
  };
  fs.writeFileSync(
    path.join(configDir, ".claude.json"),
    JSON.stringify(config, null, 2) + "\n",
  );
}

// --- Companion settings files ---

function copyCompanionSettings(script: string, configDir: string, workspace: string): string[] {
  const scriptDir = path.dirname(script);
  const scriptName = path.basename(script, ".capsh");
  const copied: string[] = [];

  // Project settings → workspace/.claude/settings.json, workspace/.claude/settings.local.json
  for (const suffix of ["settings.json", "settings.local.json"]) {
    const companion = path.join(scriptDir, `${scriptName}.${suffix}`);
    if (fs.existsSync(companion)) {
      const projectClaudeDir = path.join(workspace, ".claude");
      fs.mkdirSync(projectClaudeDir, { recursive: true });
      fs.copyFileSync(companion, path.join(projectClaudeDir, suffix));
      copied.push(suffix);
    }
  }

  // Global settings → configDir/settings.json
  const globalCompanion = path.join(scriptDir, `${scriptName}.settings.global.json`);
  if (fs.existsSync(globalCompanion)) {
    fs.copyFileSync(globalCompanion, path.join(configDir, "settings.json"));
    copied.push("settings.global.json");
  }

  return copied;
}

// --- State management ---

function sanitizeState(configDir: string): void {
  const extensions = [".json", ".jsonl", ".txt", ".md"];

  function walk(dir: string): string[] {
    const files: string[] = [];
    if (!fs.existsSync(dir)) return files;
    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      return files;
    }
    for (const entry of entries) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        files.push(...walk(full));
      } else if (entry.isFile()) {
        const matchesExt =
          extensions.some((ext) => entry.name.endsWith(ext)) ||
          entry.name.includes(".json.backup.");
        if (matchesExt) files.push(full);
      }
    }
    return files;
  }

  const replacements: [RegExp, string][] = [
    [/\/Users\/[^/]*\/Developer\//g, "/Users/alfred/"],
    [/\/Users\/[^/]*\/Desktop\//g, "/Users/alfred/"],
    [/\/Users\/[^/]*\/Documents\//g, "/Users/alfred/"],
    [/\/Users\/[^/]*\//g, "/Users/alfred/"],
    [/\/private\/var\/folders\/[^"]*/g, "/tmp/workspace"],
    [/"userID": *"[^"]*"/g, '"userID": "<user_id>"'],
    [/"accountUuid": *"[^"]*"/g, '"accountUuid": "<account_uuid>"'],
    [/"organizationUuid": *"[^"]*"/g, '"organizationUuid": "<org_uuid>"'],
    [/"emailAddress": *"[^"]*"/g, '"emailAddress": "user@example.com"'],
    [/"displayName": *"[^"]*"/g, '"displayName": "User"'],
    [/"organizationName": *"[^"]*"/g, '"organizationName": "Organization"'],
  ];

  for (const file of walk(configDir)) {
    let content = fs.readFileSync(file, "utf-8");
    for (const [pattern, replacement] of replacements) {
      content = content.replace(pattern, replacement);
    }
    fs.writeFileSync(file, content);
  }
}

function copyDirSync(src: string, dest: string): void {
  if (!fs.existsSync(src)) return;
  fs.mkdirSync(dest, { recursive: true });
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDirSync(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

function listFiles(dir: string): string[] {
  const files: string[] = [];
  function walk(d: string): void {
    if (!fs.existsSync(d)) return;
    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(d, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      const full = path.join(d, entry.name);
      if (entry.isDirectory()) {
        walk(full);
      } else if (entry.isFile()) {
        files.push(full);
      }
    }
  }
  walk(dir);
  return files.sort();
}

// --- Fixture extraction ---

function extractFixtures(framesDir: string, fixturesDir: string): string[] {
  const recording = path.join(framesDir, "recording.jsonl");
  if (!fs.existsSync(recording)) {
    process.stderr.write(
      `${RED}Error: No recording.jsonl found in ${framesDir}${NC}\n`,
    );
    return [];
  }

  fs.mkdirSync(fixturesDir, { recursive: true });

  const content = fs.readFileSync(recording, "utf-8");
  const names: string[] = [];

  for (const line of content.split("\n")) {
    const snapshotMatch = line.match(/"snapshot":"(\d+)"/);
    if (!snapshotMatch) continue;

    const frameNum = snapshotMatch[1];
    const nameMatch = line.match(/"name":"([^"]+)"/);
    if (!nameMatch) continue;

    const fixtureName = nameMatch[1];
    const plainFrame = path.join(framesDir, `${frameNum}.txt`);
    const ansiFrame = path.join(framesDir, `${frameNum}.ansi.txt`);

    if (fs.existsSync(plainFrame)) {
      fs.copyFileSync(plainFrame, path.join(fixturesDir, `${fixtureName}.tui.txt`));
      names.push(fixtureName);
      console.log(`${DIM}  ${fixtureName}${NC}`);
    } else {
      process.stderr.write(
        `${YELLOW}Warning: Frame ${frameNum} not found for ${fixtureName}${NC}\n`,
      );
    }

    if (fs.existsSync(ansiFrame)) {
      fs.copyFileSync(
        ansiFrame,
        path.join(fixturesDir, `${fixtureName}.tui.ansi.txt`),
      );
    }
  }

  if (names.length === 0) {
    process.stderr.write(`${YELLOW}Warning: No named snapshots found${NC}\n`);
  }

  return names;
}

// --- Core capture ---

async function runCapture(
  script: string,
  rawOutputBase: string,
  fixturesDir: string,
  captureTimeout: number,
  parallel: boolean,
): Promise<boolean> {
  const scriptName = path.basename(script, ".capsh");
  const rawDir = path.join(rawOutputBase, scriptName);

  fs.mkdirSync(rawDir, { recursive: true });
  fs.mkdirSync(fixturesDir, { recursive: true });

  const claudeArgs = parseClaudeArgs(script);
  const scriptEnv = parseEnv(script);
  let workspace = parseWorkspace(script);
  if (!workspace) {
    workspace = fs.mkdtempSync(path.join(os.tmpdir(), "capture-"));
  }
  // Resolve to absolute path (macOS /tmp -> /private/tmp)
  workspace = fs.realpathSync(workspace);

  const configMode = parseConfigMode(script);
  const configDir = path.join(rawDir, "state");
  let useOauthToken = true;

  switch (configMode) {
    case "trusted":
      writeClaudeConfig(configDir, workspace, VERSION);
      // CLAUDE.md suppresses the "Welcome back!" splash screen
      if (!fs.existsSync(path.join(workspace, "CLAUDE.md"))) {
        fs.writeFileSync(path.join(workspace, "CLAUDE.md"), "");
      }
      break;
    case "welcome-back":
      writeWelcomeBackConfig(configDir, workspace, VERSION);
      break;
    case "auth-only":
      writeAuthOnlyConfig(configDir, VERSION);
      break;
    case "empty":
      fs.mkdirSync(configDir, { recursive: true });
      useOauthToken = false;
      break;
  }

  // Copy companion settings files
  const companionSettings = copyCompanionSettings(script, configDir, workspace);

  // Snapshot state before capture (relative paths)
  const stateDir = path.join(rawDir, "state");
  const stateFiles = listFiles(stateDir).map((f) =>
    path.relative(rawDir, f),
  );
  fs.writeFileSync(
    path.join(rawDir, "state.before.txt"),
    stateFiles.join("\n") + "\n",
  );

  console.log(`Running: ${CYAN}${scriptName}${NC}`);
  if (claudeArgs) console.log(`  Args: ${CYAN}${claudeArgs}${NC}`);
  if (configMode !== "trusted")
    console.log(`  Config: ${MAGENTA}${configMode}${NC}`);
  if (companionSettings.length > 0)
    console.log(`  Settings: ${MAGENTA}${companionSettings.join(", ")}${NC}`);
  const envKeys = Object.keys(scriptEnv);
  if (envKeys.length > 0)
    console.log(`  Env: ${CYAN}${envKeys.map((k) => `${k}=${scriptEnv[k]}`).join(" ")}${NC}`);
  if (!parallel) console.log(`  ${DIM}Workspace: ${workspace}${NC}`);

  // Build capsh command
  const args = ["capsh", "--frames", rawDir, "--", "claude"];
  if (claudeArgs) {
    args.push(...claudeArgs.split(/\s+/));
  }

  const env: Record<string, string> = {
    ...process.env as Record<string, string>,
    ...scriptEnv,
    CLAUDE_CONFIG_DIR: configDir,
  };
  if (useOauthToken) {
    env.CLAUDE_CODE_OAUTH_TOKEN = process.env.CLAUDE_CODE_OAUTH_TOKEN ?? "";
  } else {
    env.CLAUDE_CODE_OAUTH_TOKEN = "";
  }

  const proc = Bun.spawn(["timeout", String(captureTimeout), ...args], {
    cwd: workspace,
    env,
    stdin: Bun.file(script),
    stdout: parallel ? "pipe" : "inherit",
    stderr: parallel ? "pipe" : "inherit",
  });
  const exitCode = await proc.exited;

  // Exit codes: 0=success, 143=killed by SIGTERM (expected), 124=timeout
  if (exitCode !== 0 && exitCode !== 143) {
    if (exitCode === 124) {
      process.stderr.write(
        `${RED}Error: ${scriptName} timed out after ${captureTimeout}s${NC}\n`,
      );
    } else {
      process.stderr.write(
        `${RED}Error: capsh failed for ${scriptName} (exit ${exitCode})${NC}\n`,
      );
    }
    return false;
  }

  // Sanitize state files
  sanitizeState(configDir);

  // Snapshot state after capture and generate diff
  const stateAfter = listFiles(stateDir).map((f) =>
    path.relative(rawDir, f),
  );
  fs.writeFileSync(
    path.join(rawDir, "state.after.txt"),
    stateAfter.join("\n") + "\n",
  );

  // Generate diff
  const diffResult = Bun.spawnSync(
    [
      "diff",
      path.join(rawDir, "state.before.txt"),
      path.join(rawDir, "state.after.txt"),
    ],
    { stdout: "pipe", stderr: "pipe" },
  );
  fs.writeFileSync(
    path.join(rawDir, "state.diff"),
    diffResult.stdout.toString(),
  );

  // Extract named fixtures
  const snapshotNames = extractFixtures(rawDir, fixturesDir);

  // Write manifest of TUI captures
  if (snapshotNames.length > 0) {
    const manifest = {
      script: scriptName,
      snapshots: snapshotNames,
    };
    fs.writeFileSync(
      path.join(fixturesDir, `${scriptName}.manifest.json`),
      JSON.stringify(manifest, null, 2) + "\n",
    );
  }

  // Copy state diff and state subdirectories to fixtures
  const diffFile = path.join(rawDir, "state.diff");
  if (fs.existsSync(diffFile)) {
    fs.copyFileSync(diffFile, path.join(fixturesDir, `${scriptName}.state.diff`));
  }
  for (const subdir of ["projects", "plans", "todos"]) {
    copyDirSync(
      path.join(stateDir, subdir),
      path.join(fixturesDir, `${scriptName}.${subdir}`),
    );
  }

  return true;
}

// --- Failure tracking ---

function shouldRunScript(
  scriptName: string,
  singleScript: string,
  retryMode: boolean,
  failuresFile: string,
  rawOutput: string,
): boolean {
  if (singleScript) {
    return scriptName === singleScript;
  }
  if (!retryMode) return true;

  // In retry mode: run if in failures file OR if recording doesn't exist
  if (fs.existsSync(failuresFile)) {
    const failures = fs.readFileSync(failuresFile, "utf-8");
    if (failures.split("\n").includes(scriptName)) return true;
  }
  const hasRecording = fs.existsSync(path.join(rawOutput, scriptName, "recording.jsonl"));
  const hasStdout = fs.existsSync(path.join(rawOutput, scriptName, "stdout.txt"));
  if (!hasRecording && !hasStdout) {
    return true;
  }
  return false;
}

function recordFailure(failuresFile: string, scriptName: string): void {
  fs.appendFileSync(failuresFile, scriptName + "\n");
}

function clearFailure(failuresFile: string, scriptName: string): void {
  if (!fs.existsSync(failuresFile)) return;
  const lines = fs
    .readFileSync(failuresFile, "utf-8")
    .split("\n")
    .filter((l) => l !== scriptName);
  fs.writeFileSync(failuresFile, lines.join("\n"));
}

// --- Main ---

// Parse arguments
let runSkipped = process.env.RUN_SKIPPED === "1";
let retryMode = false;
let singleScript = "";
let jobs = 1;

const argv = process.argv.slice(2);
for (let i = 0; i < argv.length; i++) {
  switch (argv[i]) {
    case "--retry":
      retryMode = true;
      break;
    case "--script":
      singleScript = argv[++i];
      if (!singleScript) {
        process.stderr.write("Error: --script requires an argument\n");
        process.exit(1);
      }
      break;
    case "-j":
    case "--jobs": {
      const val = argv[++i];
      if (!val) {
        process.stderr.write("Error: --jobs requires a number\n");
        process.exit(1);
      }
      jobs = Number(val);
      if (!Number.isInteger(jobs) || jobs < 0) {
        process.stderr.write("Error: --jobs must be a non-negative integer\n");
        process.exit(1);
      }
      if (jobs === 0) jobs = os.cpus().length;
      break;
    }
    case "--skip-skipped":
      runSkipped = false;
      break;
    case "--run-skipped":
      runSkipped = true;
      break;
    case "-h":
    case "--help":
      console.log(`Usage: capture.ts [OPTIONS]

Capture fixtures from real Claude CLI using capsh, cli, and tmux scripts.

Options:
  --script <name>         Run only the specified script (without .capsh)
  --retry                 Re-run failed scripts or scripts with missing fixtures
  -j, --jobs <n>          Run up to n captures in parallel (default: 1, 0=ncpus)
  --skip-skipped          Skip skipped scripts (default)
  --run-skipped           Run skipped scripts (may fail)
  -h, --help              Show this help

Environment variables:
  RUN_SKIPPED=1           Same as --run-skipped`);
      process.exit(0);
      break;
    default:
      process.stderr.write(`Unknown option: ${argv[i]}\n`);
      process.exit(1);
  }
}

// Check dependencies
checkClaude();
checkCapsh();

// Load OAuth token from .env
loadCaptureEnv();

// Detect version
const VERSION = detectVersion();
if (!VERSION) {
  process.stderr.write(
    `${RED}Error: Could not detect Claude CLI version${NC}\n`,
  );
  process.exit(1);
}

console.log(`${BOLD}Claude CLI version:${NC} ${GREEN}${VERSION}${NC}`);
if (jobs > 1) console.log(`${BOLD}Parallel jobs:${NC} ${GREEN}${jobs}${NC}`);

// Raw output goes in git-ignored directory
const RAW_OUTPUT = path.join(SCRIPT_DIR, "output", `v${VERSION}`);
// Fixtures go in tests/fixtures/
const FIXTURES_DIR = path.join(path.dirname(SCRIPT_DIR), "fixtures", `v${VERSION}`);
// Track failures
const FAILURES_FILE = path.join(RAW_OUTPUT, ".failures");

// Clean or preserve output based on mode
if (retryMode) {
  console.log("Retry mode: re-running failed scripts and scripts with missing fixtures");
  if (fs.existsSync(FAILURES_FILE)) {
    const count = fs
      .readFileSync(FAILURES_FILE, "utf-8")
      .split("\n")
      .filter((l) => l.trim()).length;
    console.log(`${DIM}Known failures: ${count}${NC}`);
  }
} else if (singleScript) {
  // Single script mode: clean that script's output and fixtures
  const scriptDir = path.join(RAW_OUTPUT, singleScript);
  if (fs.existsSync(scriptDir)) {
    fs.rmSync(scriptDir, { recursive: true });
  }
  // Clean stale fixture files for this script
  if (fs.existsSync(FIXTURES_DIR)) {
    // Read manifest first to find snapshot fixture names (e.g., clear_before.tui.txt)
    const manifestPath = path.join(FIXTURES_DIR, `${singleScript}.manifest.json`);
    if (fs.existsSync(manifestPath)) {
      try {
        const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf-8"));
        for (const snap of manifest.snapshots ?? []) {
          for (const suffix of [".tui.txt", ".tui.ansi.txt"]) {
            const f = path.join(FIXTURES_DIR, `${snap}${suffix}`);
            if (fs.existsSync(f)) fs.rmSync(f);
          }
        }
      } catch {}
    }
    // Remove script-prefixed files: manifest, state.diff, projects/, plans/, todos/
    for (const entry of fs.readdirSync(FIXTURES_DIR)) {
      if (entry.startsWith(`${singleScript}.`)) {
        fs.rmSync(path.join(FIXTURES_DIR, entry), { recursive: true });
      }
    }
  }
} else {
  // Clean previous output
  if (fs.existsSync(RAW_OUTPUT)) fs.rmSync(RAW_OUTPUT, { recursive: true });
  if (fs.existsSync(FIXTURES_DIR)) fs.rmSync(FIXTURES_DIR, { recursive: true });
}
fs.mkdirSync(RAW_OUTPUT, { recursive: true });
fs.mkdirSync(FIXTURES_DIR, { recursive: true });

let total = 0;
let passed = 0;
let failed = 0;
let skipped = 0;

const parallel = jobs > 1;
const semaphore = new Semaphore(jobs);
const tasks: (() => Promise<void>)[] = [];

async function runScript(
  script: string,
  defaultTimeout: number,
  allowFailure = false,
): Promise<void> {
  const scriptName = path.basename(script, ".capsh");

  if (!fs.existsSync(script)) return;

  if (
    !shouldRunScript(scriptName, singleScript, retryMode, FAILURES_FILE, RAW_OUTPUT)
  ) {
    return;
  }

  total++;

  const timeout = parseTimeout(script) ?? defaultTimeout;
  if (await runCapture(script, RAW_OUTPUT, FIXTURES_DIR, timeout, parallel)) {
    passed++;
    clearFailure(FAILURES_FILE, scriptName);
  } else {
    if (allowFailure) {
      console.log(
        `${YELLOW}Skipped script failed (expected): ${scriptName}${NC}`,
      );
      skipped++;
    } else {
      failed++;
      recordFailure(FAILURES_FILE, scriptName);
    }
  }
}

// Collect cli scripts (non-interactive, stdout capture)
console.log("\n=== Running cli scripts ===");
const cliDir = path.join(SCRIPT_DIR, "cli");
if (fs.existsSync(cliDir)) {
  const scripts = fs
    .readdirSync(cliDir)
    .filter((f) => f.endsWith(".cli"))
    .sort();
  for (const f of scripts) {
    const scriptName = path.basename(f, ".cli");
    if (
      !shouldRunScript(scriptName, singleScript, retryMode, FAILURES_FILE, RAW_OUTPUT)
    ) {
      continue;
    }
    total++;

    const scriptFile = path.join(cliDir, f);
    tasks.push(async () => {
      const content = fs.readFileSync(scriptFile, "utf-8");
      const args = content
        .split("\n")
        .filter((l) => l.trim() && !l.startsWith("#"))
        .join(" ")
        .trim()
        .split(/\s+/);

      const outDir = path.join(RAW_OUTPUT, scriptName);
      fs.mkdirSync(outDir, { recursive: true });

      console.log(`Running: ${CYAN}${scriptName}${NC}`);
      if (!parallel) console.log(`  ${DIM}claude ${args.join(" ")}${NC}`);

      const proc = Bun.spawn(["claude", ...args], {
        stdout: "pipe",
        stderr: "pipe",
        env: process.env as Record<string, string>,
      });
      const exitCode = await proc.exited;

      const stdout = await new Response(proc.stdout).text();
      const stderr = await new Response(proc.stderr).text();

      fs.writeFileSync(path.join(outDir, "stdout.txt"), stdout);
      if (stderr) {
        fs.writeFileSync(path.join(outDir, "stderr.txt"), stderr);
      }

      // Copy stdout as the fixture
      fs.mkdirSync(FIXTURES_DIR, { recursive: true });
      fs.writeFileSync(path.join(FIXTURES_DIR, `${scriptName}.cli.txt`), stdout);

      if (exitCode === 0) {
        passed++;
        clearFailure(FAILURES_FILE, scriptName);
      } else {
        process.stderr.write(
          `${RED}Error: cli script failed for ${scriptName} (exit ${exitCode})${NC}\n`,
        );
        if (stderr) process.stderr.write(`${DIM}${stderr}${NC}\n`);
        failed++;
        recordFailure(FAILURES_FILE, scriptName);
      }
    });
  }
}

// Collect capsh scripts
console.log("\n=== Running capsh scripts ===");
const capshDir = path.join(SCRIPT_DIR, "capsh");
if (fs.existsSync(capshDir)) {
  const scripts = fs
    .readdirSync(capshDir)
    .filter((f) => f.endsWith(".capsh"))
    .sort();
  for (const f of scripts) {
    const scriptPath = path.join(capshDir, f);
    tasks.push(() => runScript(scriptPath, KEYBOARD_TIMEOUT));
  }
}

// Collect tmux scripts
console.log("\n=== Running tmux scripts ===");
const tmuxDir = path.join(SCRIPT_DIR, "tmux");
if (fs.existsSync(tmuxDir)) {
  const scripts = fs
    .readdirSync(tmuxDir)
    .filter((f) => f.endsWith(".sh"))
    .sort();
  for (const f of scripts) {
    const scriptName = path.basename(f, ".sh");
    if (
      !shouldRunScript(scriptName, singleScript, retryMode, FAILURES_FILE, RAW_OUTPUT)
    ) {
      continue;
    }
    total++;
    const scriptPath = path.join(tmuxDir, f);
    tasks.push(async () => {
      console.log(`Running: ${CYAN}${scriptName}${NC}`);
      const proc = Bun.spawn(["bash", scriptPath], {
        stdout: parallel ? "pipe" : "inherit",
        stderr: parallel ? "pipe" : "inherit",
        env: process.env as Record<string, string>,
      });
      const exitCode = await proc.exited;
      if (exitCode === 0) {
        passed++;
        clearFailure(FAILURES_FILE, scriptName);
      } else {
        failed++;
        recordFailure(FAILURES_FILE, scriptName);
      }
    });
  }
}

// Collect skipped scripts
if (runSkipped) {
  console.log(`\n${YELLOW}=== Running skipped scripts (may fail) ===${NC}`);
  const skippedDir = path.join(SCRIPT_DIR, "skip");
  if (fs.existsSync(skippedDir)) {
    const scripts = fs
      .readdirSync(skippedDir)
      .filter((f) => f.endsWith(".capsh"))
      .sort();
    for (const f of scripts) {
      const scriptPath = path.join(skippedDir, f);
      tasks.push(() => runScript(scriptPath, THINKING_TIMEOUT, true));
    }
  }
} else {
  console.log("\n=== Skipping skipped scripts ===");
}

// Execute all tasks with semaphore
await Promise.all(tasks.map((task) => semaphore.run(task)));

// Summary
console.log("\n=== Summary ===");
console.log(`Total: ${total}`);
console.log(`${GREEN}Passed: ${passed}${NC}`);
console.log(`${YELLOW}Skipped: ${skipped}${NC}`);
if (failed > 0) {
  console.log(`${RED}Failed: ${failed}${NC}`);
  console.log(`${DIM}Run with --retry to re-run only failed scripts${NC}`);
  process.exit(1);
} else {
  console.log(`Failed: ${failed}`);
  // Clear failures file on full success
  if (fs.existsSync(FAILURES_FILE)) fs.unlinkSync(FAILURES_FILE);
}

console.log(`\nRaw output: ${RAW_OUTPUT}`);
console.log(`Fixtures: ${FIXTURES_DIR}`);
