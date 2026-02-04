// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Extract conversation turns, tool calls, and fixture data from capture output.
//!
//! This module handles parsing session JSONL files and TUI fixture snapshots
//! to reconstruct conversation turns with their tool call chains.

import * as fs from "node:fs";
import * as path from "node:path";

// --- Exported types ---

export interface ExtractedToolCall {
  toolName: string;
  toolUseId: string;
  input: Record<string, any>;
  result: string | null;
  isError: boolean;
  /** True if this is a synthetic context read injected from fixture, not from JSONL. */
  isContextRead?: boolean;
}

export interface ResponseStep {
  text: string;
  toolCalls: ExtractedToolCall[];
}

export interface ConversationTurn {
  prompt: string;
  steps: ResponseStep[];
  timestamp: string;
}

export interface CapshManifest {
  script: string;
  snapshots: string[];
}

// --- Internal types ---

interface JsonlEntry {
  type: string;
  uuid?: string;
  parentUuid?: string | null;
  userType?: string;
  isMeta?: boolean;
  isApiErrorMessage?: boolean;
  isSidechain?: boolean;
  cwd?: string;
  message?: {
    role: string;
    content: string | Array<{ type: string; text?: string; name?: string; id?: string; input?: any; tool_use_id?: string; content?: string; is_error?: boolean; thinking?: string }>;
  };
  timestamp?: string;
  toolUseResult?: any;
}

// --- Extract placeholder text from fixture files ---

/**
 * Extract the placeholder text from a TUI fixture file.
 * Looks for `❯\xa0Try "..."` or `!\xa0Try "..."` patterns.
 */
export function extractPlaceholder(fixturesDir: string, snapshotName: string): string | null {
  const fixturePath = path.join(fixturesDir, `${snapshotName}.tui.txt`);
  if (!fs.existsSync(fixturePath)) return null;

  const content = fs.readFileSync(fixturePath, "utf-8");
  // Match ❯ or ! followed by NBSP or space, then the placeholder text
  const match = content.match(/[❯!][\s\u00A0](Try "[^"]*")/);
  return match ? match[1] : null;
}

/**
 * Extract the welcome-back right panel rows from a TUI fixture file.
 * The right panel is inside the welcome box, after the │ separator.
 */
export function extractWelcomeBackRightPanel(fixturesDir: string, snapshotName: string): string[] | null {
  const fixturePath = path.join(fixturesDir, `${snapshotName}.tui.txt`);
  if (!fs.existsSync(fixturePath)) return null;

  const content = fs.readFileSync(fixturePath, "utf-8");
  if (!content.includes("Welcome back!")) return null;

  const lines = content.split("\n");
  const rows: string[] = [];

  for (const line of lines) {
    // Match lines inside the welcome box with a right panel
    // Format: │ ... │ Right panel text     │
    const boxMatch = line.match(/^│.*│\s(.{23,25})\s*│$/);
    if (!boxMatch) continue;

    const rightText = boxMatch[1].trimEnd();

    if (rightText.match(/^\s*$/)) {
      rows.push("");
      continue;
    }

    if (rightText.match(/^─+$/)) {
      rows.push("---");
      continue;
    }

    rows.push(rightText.trim());
  }

  if (rows.length === 0) return null;

  // Trim trailing empty rows
  while (rows.length > 0 && rows[rows.length - 1] === "") {
    rows.pop();
  }

  return rows.length > 0 ? rows : null;
}

// --- Internal helpers ---

/** Check if a user message contains tool_result blocks (not a real user prompt). */
function isToolResultMessage(entry: JsonlEntry): boolean {
  if (entry.type !== "user") return false;
  const content = entry.message?.content;
  if (!Array.isArray(content)) return false;
  return content.some((b) => b.type === "tool_result");
}

/**
 * Attempt to salvage uuid/parentUuid/type from a malformed JSONL line.
 * Some lines have broken JSON in thinking content. We extract the key fields
 * so the chain-building logic can still connect parent→child relationships.
 */
function salvageMalformedEntry(line: string): JsonlEntry | null {
  const uuidMatch = line.match(/"uuid"\s*:\s*"([^"]+)"/);
  const parentMatch = line.match(/"parentUuid"\s*:\s*"([^"]+)"/);
  const typeMatch = line.match(/"type"\s*:\s*"(user|assistant|progress|file-history-snapshot)"/);

  if (!uuidMatch || !typeMatch) return null;

  return {
    type: typeMatch[1],
    uuid: uuidMatch[1],
    parentUuid: parentMatch ? parentMatch[1] : null,
    timestamp: (line.match(/"timestamp"\s*:\s*"([^"]+)"/) || [])[1],
  };
}

/**
 * Extract a likely filename from user prompt text.
 * Matches patterns like "file called X", "file named X", "the file X",
 * "Read the file X", etc.
 */
function extractFilenameFromPrompt(prompt: string): string | null {
  const patterns = [
    /file\s+called\s+(\S+)/i,
    /file\s+named\s+(\S+)/i,
    /(?:Read|read|Write|write|Edit|edit|Create|create)\s+(?:a\s+)?(?:new\s+)?(?:file\s+)?(?:called\s+)?(\S*\.[\w-]+)/i,
    /the\s+file\s+(\S*\.[\w-]+)/i,
  ];
  for (const pattern of patterns) {
    const match = prompt.match(pattern);
    if (match) return match[1];
  }
  return null;
}

/**
 * Normalize a file path from JSONL tool inputs.
 * Strips the capture workspace cwd prefix to make paths relative.
 * If file_path equals cwd exactly (model gave directory as path), tries to
 * find the actual filename from file-history-snapshots or the user prompt.
 */
function normalizeFilePath(filePath: string, cwd: string | null, fileHistoryNames: string[], promptText: string | null): string {
  if (!cwd) return filePath;

  // Strip cwd prefix + "/" to get relative path
  const cwdWithSlash = cwd.endsWith("/") ? cwd : cwd + "/";
  if (filePath.startsWith(cwdWithSlash)) {
    return filePath.slice(cwdWithSlash.length);
  }

  // file_path equals cwd exactly — model gave directory as file path
  if (filePath === cwd) {
    // Try to find the actual filename from file-history-snapshots
    if (fileHistoryNames.length > 0) {
      return fileHistoryNames[fileHistoryNames.length - 1];
    }
    // Try to extract from user prompt
    if (promptText) {
      const extracted = extractFilenameFromPrompt(promptText);
      if (extracted) return extracted;
    }
    // Can't determine — return as-is
    return filePath;
  }

  return filePath;
}

/**
 * Normalize file_path values in tool call inputs.
 */
function normalizeToolInput(input: Record<string, any>, cwd: string | null, fileHistoryNames: string[], promptText: string | null): Record<string, any> {
  const result = { ...input };
  if (typeof result.file_path === "string") {
    result.file_path = normalizeFilePath(result.file_path, cwd, fileHistoryNames, promptText);
  }
  return result;
}

/** Strip <system-reminder>...</system-reminder> tags from tool result text. */
function stripSystemReminders(text: string): string {
  return text.replace(/<system-reminder>[\s\S]*?<\/system-reminder>/g, "").trim();
}

/** Strip markdown code fences from response text (```...```). */
function stripCodeFences(text: string): string {
  // Remove wrapping code fences: ```\ncontent\n```  →  content
  let result = text.replace(/^```\w*\n([\s\S]*?)\n```$/g, "$1").trim();
  // Strip Read tool arrow prefixes (→) from lines
  result = result.replace(/^→/gm, "");
  return result;
}

/**
 * Strip Read tool line-number prefixes from tool result text.
 * Read results come formatted as "     1→content\n     2→more\n".
 * Strip the "     N→" prefix from each line to get the raw file content.
 */
function stripReadLineNumbers(text: string): string {
  return text.replace(/^ *\d+\u2192/gm, "");
}

/** Extract text from a user message content field (string or array). */
function extractMessageText(content: string | Array<{ type: string; text?: string }> | undefined): string | null {
  if (!content) return null;
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    const texts = content
      .filter((b) => b.type === "text" && b.text)
      .map((b) => b.text!);
    return texts.length > 0 ? texts.join("\n") : null;
  }
  return null;
}

/**
 * Clean up a user prompt for pattern matching:
 * - Strip embedded slash commands (e.g., "\n/compact" at end of prompt)
 * - Strip control characters (e.g., Ctrl-U = \u0015)
 * - Trim whitespace
 */
function cleanPrompt(prompt: string): string {
  // Remove control characters (except newline)
  let cleaned = prompt.replace(/[\x00-\x09\x0B-\x1F\x7F]/g, "");

  // Split lines and remove lines that are slash commands
  const lines = cleaned.split("\n");
  const kept = lines.filter((line) => !line.trim().match(/^\/\w+/));
  cleaned = kept.join("\n").trim();

  return cleaned;
}

// --- Extract conversation turns from session JSONL files ---

/**
 * Extract user→assistant conversation turns from session JSONL files.
 * Reads all .jsonl files from {fixturesDir}/{scriptName}.projects/,
 * skipping subagents/, meta messages, errors, commands, and interrupts.
 *
 * Extracts full tool-use chains as ResponseStep[] from JSONL data.
 */
export function extractConversationTurns(fixturesDir: string, scriptName: string): ConversationTurn[] {
  const projectsDir = path.join(fixturesDir, `${scriptName}.projects`);
  if (!fs.existsSync(projectsDir)) return [];

  // Find all session JSONL files (skip subagents/)
  const jsonlFiles: string[] = [];
  for (const projDir of fs.readdirSync(projectsDir)) {
    const projPath = path.join(projectsDir, projDir);
    if (!fs.statSync(projPath).isDirectory()) continue;
    for (const file of fs.readdirSync(projPath)) {
      if (file.endsWith(".jsonl")) {
        const fullPath = path.join(projPath, file);
        if (!fullPath.includes("/subagents/")) {
          jsonlFiles.push(fullPath);
        }
      }
    }
  }

  const allTurns: ConversationTurn[] = [];

  for (const file of jsonlFiles) {
    const content = fs.readFileSync(file, "utf-8");
    const lines = content.split("\n").filter((l) => l.trim());

    // Parse all entries
    const entries: JsonlEntry[] = [];
    for (const line of lines) {
      try {
        const entry: JsonlEntry = JSON.parse(line);
        entries.push(entry);
      } catch {
        // Some JSONL lines have malformed thinking content with unescaped quotes.
        // Salvage uuid/parentUuid/type so chain-building still works.
        const salvaged = salvageMalformedEntry(line);
        if (salvaged) entries.push(salvaged);
      }
    }

    // Build uuid→entry map and extract cwd
    const byUuid = new Map<string, JsonlEntry>();
    let sessionCwd: string | null = null;
    for (const entry of entries) {
      if (entry.uuid) {
        byUuid.set(entry.uuid, entry);
      }
      if (entry.cwd && !sessionCwd) {
        sessionCwd = entry.cwd;
      }
    }

    // First pass: identify prompt user messages (real user input, not tool results)
    const promptUuids: string[] = [];
    for (const entry of entries) {
      if (entry.type !== "user" || !entry.uuid) continue;
      if (entry.isMeta) continue;
      if (entry.isSidechain) continue;
      if (entry.userType && entry.userType !== "external") continue;
      if (isToolResultMessage(entry)) continue;

      const text = extractMessageText(entry.message?.content);
      if (!text) continue;

      // Skip command messages
      if (text.includes("<command-name>") || text.includes("<local-command-caveat>") || text.includes("<local-command-stdout>")) continue;
      // Skip interrupted messages
      if (text.includes("[Request interrupted by user")) continue;
      // Skip compact summaries
      if ((entry as any).isCompactSummary) continue;
      // Skip visible-in-transcript-only messages
      if ((entry as any).isVisibleInTranscriptOnly) continue;

      promptUuids.push(entry.uuid);
    }

    // Second pass: for each prompt, build forward chain and extract steps
    for (const promptUuid of promptUuids) {
      const promptEntry = byUuid.get(promptUuid)!;
      const promptText = extractMessageText(promptEntry.message?.content);
      if (!promptText) continue;

      // Build forward chain: all uuids reachable from promptUuid via parentUuid
      const chainUuids = new Set<string>([promptUuid]);
      for (const entry of entries) {
        if (!entry.uuid) continue;
        if (entry.parentUuid && chainUuids.has(entry.parentUuid)) {
          chainUuids.add(entry.uuid);
        }
      }

      // Collect chain entries in order (excluding the prompt itself)
      const chainEntries = entries.filter(
        (e) => e.uuid && chainUuids.has(e.uuid) && e.uuid !== promptUuid
      );

      // Collect file-history-snapshot filenames from ALL entries for path resolution.
      // File-history-snapshots don't have uuid/parentUuid, so they're not in the chain.
      const fileHistoryNames: string[] = [];
      for (const entry of entries) {
        if (entry.type === "file-history-snapshot") {
          const snapshot = (entry as any).snapshot;
          if (snapshot?.trackedFileBackups) {
            for (const name of Object.keys(snapshot.trackedFileBackups)) {
              if (!fileHistoryNames.includes(name)) {
                fileHistoryNames.push(name);
              }
            }
          }
        }
      }

      // Walk chain to build ResponseStep[]
      const steps: ResponseStep[] = [];
      let currentStep: ResponseStep = { text: "", toolCalls: [] };
      const toolCallsById = new Map<string, ExtractedToolCall>();

      for (const entry of chainEntries) {
        if (entry.type === "file-history-snapshot" || entry.type === "progress") continue;
        if (entry.isSidechain) continue;
        if (entry.isMeta) continue;
        if (entry.isApiErrorMessage) continue;

        if (entry.type === "assistant" && entry.message?.content) {
          const content = entry.message.content;
          if (typeof content === "string") {
            if (currentStep.text) currentStep.text += "\n";
            currentStep.text += content;
            continue;
          }
          if (Array.isArray(content)) {
            for (const block of content) {
              if (block.type === "thinking") continue;
              if (block.type === "text" && block.text) {
                if (currentStep.text) currentStep.text += "\n";
                currentStep.text += block.text;
              }
              if (block.type === "tool_use" && block.name && block.id) {
                const tc: ExtractedToolCall = {
                  toolName: block.name,
                  toolUseId: block.id,
                  input: normalizeToolInput(block.input || {}, sessionCwd, fileHistoryNames, promptText),
                  result: null,
                  isError: false,
                };
                currentStep.toolCalls.push(tc);
                toolCallsById.set(block.id, tc);
              }
            }
          }
        }

        if (entry.type === "user" && entry.message?.content) {
          const content = entry.message.content;

          // String content = real user prompt or meta message — stop chain
          if (typeof content === "string") {
            if (entry.isMeta) continue;
            if (content.includes("<command-name>") || content.includes("<local-command-caveat>") || content.includes("<local-command-stdout>")) continue;
            if (content.includes("[Request interrupted by user")) continue;
            // Real user prompt — new turn started, stop here
            break;
          }

          if (!Array.isArray(content)) continue;

          // Check for tool_result blocks
          const hasToolResult = content.some((b) => b.type === "tool_result");
          if (hasToolResult) {
            for (const block of content) {
              if (block.type === "tool_result" && block.tool_use_id) {
                const tc = toolCallsById.get(block.tool_use_id);
                if (tc) {
                  let resultText = block.content ? stripSystemReminders(block.content) : null;
                  if (resultText && tc.toolName === "Read") {
                    resultText = stripReadLineNumbers(resultText);
                  }
                  // Normalize cwd paths in result text
                  if (resultText && sessionCwd) {
                    resultText = resultText.replace(new RegExp(sessionCwd.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "/?", "g"), "");
                  }
                  tc.result = resultText;
                  tc.isError = block.is_error === true;
                }
              }
            }
            // Push current step and start new one (tool result triggers new API turn)
            steps.push(currentStep);
            currentStep = { text: "", toolCalls: [] };
            continue;
          }

          // Check for interrupt messages - skip them
          const text = extractMessageText(content);
          if (text && text.includes("[Request interrupted by user")) continue;
          // Skip command messages that appear in the chain
          if (text && (text.includes("<command-name>") || text.includes("<local-command-caveat>") || text.includes("<local-command-stdout>"))) continue;

          // If this is a real user prompt (not tool_result, not interrupt, not command),
          // it means a new conversation turn started — stop building steps for this prompt.
          if (text) break;
        }
      }

      // Push final step if non-empty
      if (currentStep.text || currentStep.toolCalls.length > 0) {
        steps.push(currentStep);
      }

      // Collapse: if the last step is text-only (no tool calls) and there are
      // earlier steps with tool calls, merge the text into the first step.
      // This matches how Claude responses render: tool calls + final text in one response.
      if (steps.length >= 2) {
        const lastStep = steps[steps.length - 1];
        if (lastStep.text && lastStep.toolCalls.length === 0) {
          steps[0].text = lastStep.text;
          steps.pop();
        }
      }

      // Strip markdown code fences from response text in steps with tool calls
      for (const step of steps) {
        if (step.toolCalls.length > 0 && step.text) {
          step.text = stripCodeFences(step.text);
        }
      }

      // Skip turns with no meaningful content
      if (steps.length === 0) continue;
      if (steps.every((s) => !s.text && s.toolCalls.length === 0)) continue;

      const cleaned = cleanPrompt(promptText);
      if (!cleaned) continue;

      allTurns.push({
        prompt: cleaned,
        steps,
        timestamp: promptEntry.timestamp || "",
      });
    }
  }

  // Deduplicate by prompt content (keep last occurrence from retries)
  const seen = new Map<string, ConversationTurn>();
  for (const turn of allTurns) {
    seen.set(turn.prompt, turn);
  }

  // Sort by timestamp to maintain conversation order
  const result = [...seen.values()];
  result.sort((a, b) => a.timestamp.localeCompare(b.timestamp));

  return result;
}

// --- Detect context reads from fixtures ---

/**
 * Scan fixture TUI snapshots for context reads ("Reading N file…" or "Read N file")
 * that appear before permission dialogs. These are Claude Code's internal context
 * gathering and are NOT logged as tool_use in the JSONL. Returns a map from
 * turn index to the context read result string to inject.
 */
export function detectContextReadsFromFixtures(
  fixturesDir: string,
  manifest: CapshManifest | null,
  turns: ConversationTurn[],
): Map<number, string> {
  const contextReads = new Map<number, string>();
  if (!manifest) return contextReads;

  for (const snap of manifest.snapshots) {
    const fixturePath = path.join(fixturesDir, `${snap}.tui.txt`);
    if (!fs.existsSync(fixturePath)) continue;

    const content = fs.readFileSync(fixturePath, "utf-8");

    // Look for "⏺ Reading N file…" patterns (context reads in progress)
    const readingMatch = content.match(/⏺ Reading (\d+ file\S*)/);
    if (!readingMatch) continue;

    // Find which turn this context read belongs to by checking if the fixture
    // contains the prompt text for a turn that has tool calls but no Read
    for (let i = 0; i < turns.length; i++) {
      const turn = turns[i];
      const firstStep = turn.steps[0];
      if (!firstStep || firstStep.toolCalls.length === 0) continue;

      // Check if turn already has a Read tool call
      const hasRead = firstStep.toolCalls.some((tc) => tc.toolName === "Read");
      if (hasRead) continue;

      // Check if the fixture contains this turn's prompt
      if (content.includes(turn.prompt.slice(0, 40))) {
        // Inject context read result with trailing ellipsis for "Reading" display
        contextReads.set(i, readingMatch[1]);
        break;
      }
    }
  }

  return contextReads;
}
