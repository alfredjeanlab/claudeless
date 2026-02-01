#!/usr/bin/env bun

import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";

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

function parseConfigMode(script: string): "trusted" | "auth-only" | "empty" {
  const content = fs.readFileSync(script, "utf-8");
  const match = content.match(/^# Config:\s*(.*)$/m);
  if (match) {
    const mode = match[1].trim().split(/\s+/)[0];
    if (mode === "auth-only" || mode === "empty") return mode;
  }
  return "trusted";
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

// --- State management ---

function sanitizeState(configDir: string): void {
  const extensions = [".json", ".jsonl", ".txt", ".md"];

  function walk(dir: string): string[] {
    const files: string[] = [];
    if (!fs.existsSync(dir)) return files;
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
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

function listFiles(dir: string): string[] {
  const files: string[] = [];
  function walk(d: string): void {
    if (!fs.existsSync(d)) return;
    for (const entry of fs.readdirSync(d, { withFileTypes: true })) {
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

function extractFixtures(framesDir: string, fixturesDir: string): void {
  const recording = path.join(framesDir, "recording.jsonl");
  if (!fs.existsSync(recording)) {
    process.stderr.write(
      `${RED}Error: No recording.jsonl found in ${framesDir}${NC}\n`,
    );
    return;
  }

  fs.mkdirSync(fixturesDir, { recursive: true });

  const content = fs.readFileSync(recording, "utf-8");
  let count = 0;

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
      fs.copyFileSync(plainFrame, path.join(fixturesDir, `${fixtureName}.txt`));
      count++;
      console.log(`${DIM}  ${fixtureName}${NC}`);
    } else {
      process.stderr.write(
        `${YELLOW}Warning: Frame ${frameNum} not found for ${fixtureName}${NC}\n`,
      );
    }

    if (fs.existsSync(ansiFrame)) {
      fs.copyFileSync(
        ansiFrame,
        path.join(fixturesDir, `${fixtureName}.ansi.txt`),
      );
    }
  }

  if (count === 0) {
    process.stderr.write(`${YELLOW}Warning: No named snapshots found${NC}\n`);
  }
}

// --- Core capture ---

function runCapture(
  script: string,
  rawOutputBase: string,
  fixturesDir: string,
  captureTimeout: number,
): boolean {
  const scriptName = path.basename(script, ".capsh");
  const rawDir = path.join(rawOutputBase, scriptName);

  fs.mkdirSync(rawDir, { recursive: true });
  fs.mkdirSync(fixturesDir, { recursive: true });

  const claudeArgs = parseClaudeArgs(script);
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
      break;
    case "auth-only":
      writeAuthOnlyConfig(configDir, VERSION);
      break;
    case "empty":
      fs.mkdirSync(configDir, { recursive: true });
      useOauthToken = false;
      break;
  }

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
  console.log(`  ${DIM}Workspace: ${workspace}${NC}`);

  // Build capsh command
  const args = ["capsh", "--frames", rawDir, "--", "claude"];
  if (claudeArgs) {
    args.push(...claudeArgs.split(/\s+/));
  }

  const env: Record<string, string> = {
    ...process.env as Record<string, string>,
    CLAUDE_CONFIG_DIR: configDir,
  };
  if (useOauthToken) {
    env.CLAUDE_CODE_OAUTH_TOKEN = process.env.CLAUDE_CODE_OAUTH_TOKEN ?? "";
  } else {
    env.CLAUDE_CODE_OAUTH_TOKEN = "";
  }

  const scriptFile = fs.openSync(script, "r");
  const proc = Bun.spawnSync(["timeout", String(captureTimeout), ...args], {
    cwd: workspace,
    env,
    stdin: scriptFile,
    stdout: "inherit",
    stderr: "inherit",
  });
  fs.closeSync(scriptFile);

  const exitCode = proc.exitCode ?? 1;

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
  extractFixtures(rawDir, fixturesDir);

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
  if (!fs.existsSync(path.join(rawOutput, scriptName, "recording.jsonl"))) {
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
    case "--skip-skipped":
      runSkipped = false;
      break;
    case "--run-skipped":
      runSkipped = true;
      break;
    case "-h":
    case "--help":
      console.log(`Usage: capture.ts [OPTIONS]

Capture TUI fixtures from real Claude CLI using capsh scripts.

Options:
  --script <name>         Run only the specified script (without .capsh)
  --retry                 Re-run failed scripts or scripts with missing fixtures
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
  // Single script mode: only clean that script's output
  const scriptDir = path.join(RAW_OUTPUT, singleScript);
  if (fs.existsSync(scriptDir)) {
    fs.rmSync(scriptDir, { recursive: true });
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

function runScript(
  script: string,
  timeout: number,
  allowFailure = false,
): void {
  const scriptName = path.basename(script, ".capsh");

  if (!fs.existsSync(script)) return;

  if (
    !shouldRunScript(scriptName, singleScript, retryMode, FAILURES_FILE, RAW_OUTPUT)
  ) {
    return;
  }

  total++;

  if (runCapture(script, RAW_OUTPUT, FIXTURES_DIR, timeout)) {
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

// Run capsh scripts
console.log("\n=== Running capsh scripts ===");
const capshDir = path.join(SCRIPT_DIR, "capsh");
if (fs.existsSync(capshDir)) {
  const scripts = fs
    .readdirSync(capshDir)
    .filter((f) => f.endsWith(".capsh"))
    .sort();
  for (const f of scripts) {
    runScript(path.join(capshDir, f), KEYBOARD_TIMEOUT);
  }
}

// Run tmux scripts
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
    const result = Bun.spawnSync(["bash", scriptPath], {
      stdout: "inherit",
      stderr: "inherit",
      env: process.env as Record<string, string>,
    });
    if (result.exitCode === 0) {
      passed++;
      clearFailure(FAILURES_FILE, scriptName);
    } else {
      failed++;
      recordFailure(FAILURES_FILE, scriptName);
    }
  }
}

// Run skipped scripts
if (runSkipped) {
  console.log(`\n${YELLOW}=== Running skipped scripts (may fail) ===${NC}`);
  const skippedDir = path.join(SCRIPT_DIR, "skipped");
  if (fs.existsSync(skippedDir)) {
    const scripts = fs
      .readdirSync(skippedDir)
      .filter((f) => f.endsWith(".capsh"))
      .sort();
    for (const f of scripts) {
      runScript(path.join(skippedDir, f), THINKING_TIMEOUT, true);
    }
  }
} else {
  console.log("\n=== Skipping skipped scripts ===");
}

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
