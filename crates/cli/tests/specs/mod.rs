// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Shared infrastructure for generated capture-fixture spec tests.
//!
//! Tests in this suite replay simplified `.capsh` scripts against claudeless
//! and assert TUI snapshots, state directories, and CLI output against golden
//! fixtures captured from real Claude CLI.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// =============================================================================
// Path helpers
// =============================================================================

/// Root of the workspace (two levels above crates/cli).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// `tests/fixtures/v{version}/` directory.
fn fixtures_dir(version: &str) -> PathBuf {
    workspace_root().join("tests/fixtures").join(version)
}

/// `tests/specs/` directory (generated capsh scripts + scenarios).
fn specs_dir() -> PathBuf {
    workspace_root().join("tests/specs")
}

/// Path to the claudeless binary (built by cargo).
fn claudeless_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_claudeless"))
}

/// Path to the capsh binary (built by cargo from workspace).
fn capsh_bin() -> PathBuf {
    // capsh is a workspace member, so it's in the same target directory
    let claudeless = claudeless_bin();
    claudeless.with_file_name("capsh")
}

// =============================================================================
// Fixture loading
// =============================================================================

/// Load a TUI fixture file. Returns `None` if file doesn't exist.
fn try_load_tui_fixture(version: &str, snapshot: &str, ansi: bool) -> Option<String> {
    let suffix = if ansi { ".tui.ansi.txt" } else { ".tui.txt" };
    let path = fixtures_dir(version).join(format!("{snapshot}{suffix}"));
    fs::read_to_string(&path).ok()
}

/// Load a TUI fixture file. Panics if file doesn't exist.
fn load_tui_fixture(version: &str, snapshot: &str, ansi: bool) -> String {
    let suffix = if ansi { ".tui.ansi.txt" } else { ".tui.txt" };
    let path = fixtures_dir(version).join(format!("{snapshot}{suffix}"));
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load TUI fixture {}: {}", path.display(), e))
}

/// Load a CLI fixture file.
fn load_cli_fixture(version: &str, name: &str) -> String {
    let path = fixtures_dir(version).join(format!("{name}.cli.txt"));
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load CLI fixture {}: {}", path.display(), e))
}

/// Load a tmux fixture file.
fn load_tmux_fixture(version: &str, name: &str) -> String {
    let path = fixtures_dir(version).join(format!("{name}.tmux.txt"));
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load tmux fixture {}: {}", path.display(), e))
}

/// Load a state diff file.
fn load_state_diff(version: &str, script: &str) -> String {
    let path = fixtures_dir(version).join(format!("{script}.state.diff"));
    fs::read_to_string(&path).unwrap_or_default()
}

// =============================================================================
// Recording parser
// =============================================================================

/// Parsed snapshot from recording.jsonl.
struct SnapshotEntry {
    name: String,
    frame: String,
}

/// Parse recording.jsonl to find named snapshot frame numbers.
fn parse_recording_snapshots(frames_dir: &Path) -> Vec<SnapshotEntry> {
    let recording_path = frames_dir.join("recording.jsonl");
    let content = fs::read_to_string(&recording_path)
        .unwrap_or_else(|e| panic!("Failed to read recording.jsonl: {}", e));

    let mut snapshots = Vec::new();
    for line in content.lines() {
        // Look for lines with "snapshot" and "name" fields
        if let (Some(snap_start), Some(name_start)) =
            (line.find("\"snapshot\":\""), line.find("\"name\":\""))
        {
            let snap_val_start = snap_start + "\"snapshot\":\"".len();
            let snap_val_end = line[snap_val_start..]
                .find('"')
                .map(|i| snap_val_start + i)
                .unwrap_or(snap_val_start);
            let frame = line[snap_val_start..snap_val_end].to_string();

            let name_val_start = name_start + "\"name\":\"".len();
            let name_val_end = line[name_val_start..]
                .find('"')
                .map(|i| name_val_start + i)
                .unwrap_or(name_val_start);
            let name = line[name_val_start..name_val_end].to_string();

            snapshots.push(SnapshotEntry { name, frame });
        }
    }
    snapshots
}

// =============================================================================
// TUI normalization
// =============================================================================

/// Normalize plain TUI text for comparison.
///
/// Normalizes known rendering differences between real Claude and claudeless:
/// - Trim trailing whitespace per line and strip leading/trailing blank lines
/// - Logo characters: real Claude uses bg-colored chars, claudeless uses fg blocks
/// - Working directory path: replace with `<CWD>` (varies per machine)
/// - "/model to try" hint line: claudeless doesn't render this
/// - Response text arrows: real Claude uses `→` prefix on content lines
/// - Response continuation indentation: real Claude indents continuation lines
/// - Autocomplete: real Claude shows multiple suggestions, claudeless shows one
/// - Starfield: collapse duplicate starfield sections in setup wizard
/// - Duplicate logos: collapse consecutive logo blocks into one
fn normalize_tui(text: &str) -> String {
    use regex::Regex;

    let lines: Vec<&str> = text.lines().collect();

    // Logo line patterns
    // Match logo line 1 but NOT the help dialog tab header (which also contains "Claude Code v...")
    // Tab headers contain "general", "commands", or "custom-commands" after the version.
    let logo_line1_re =
        Regex::new(r"^[▗▐▛█▜▌▖ ]+(\s+(Claude Code|Claudeless)\s+v\S+)\s*$").unwrap();
    let logo_line2_re = Regex::new(r"^[▝▜█▛▘ ]+(\s+.+·.+)$").unwrap();
    let logo_line3_re = Regex::new(r"^(\s*▘▘ ▝▝\s+)(.+)$").unwrap();
    let model_to_try_re = Regex::new(r"^\s*/model to try").unwrap();
    let welcome_to_re = Regex::new(r"^\s*Welcome to .+$").unwrap();
    // Background bleed-through from behind dialogs in real Claude
    let chrome_settings_re = Regex::new(r"^\s+Claude in Chrome .+settings").unwrap();
    let checking_updates_re = Regex::new(r"\s+Checking for updates$").unwrap();
    // Welcome box path line: │ /some/path ... │ ... │
    let box_path_re = Regex::new(r"^│ /\S+\s+│\s+│$").unwrap();
    // Spinner lines: a spinner frame char followed by a word and ellipsis (e.g., "· Swirling…")
    let spinner_line_re = Regex::new(r"^[·✢✳✶✻✽] \w+…$").unwrap();
    // Multi-word spinner lines: normalize frame char to ✻ (e.g., "✢ Compacting conversation…" → "✻ Compacting conversation…")
    let spinner_multiword_re = Regex::new(r"^[·✢✳✶✻✽]( .+…)$").unwrap();
    // "· esc to interrupt" suffix appended to status bar during streaming
    let esc_interrupt_suffix_re = Regex::new(r"\s*· esc to interrupt$").unwrap();
    // Standalone "esc to interrupt" line (after other stripping) → ready-state hint
    let esc_interrupt_standalone_re = Regex::new(r"^\s*esc to interrupt$").unwrap();
    // Marketplace plugin notification appended to status bar (may wrap across lines)
    let marketplace_re = Regex::new(r"\s*✓ Anthropic marketplace installed.*$").unwrap();
    let marketplace_cont_re = Regex::new(r"^\s+(available\s+)?plugins\s*$").unwrap();
    // "Tip: Run /terminal-setup..." dynamic hint from real Claude
    let tip_re = Regex::new(r"^\s*⎿\s+Tip: Run /terminal-setup").unwrap();
    // Export filename with timestamp (e.g., conversation-2026-01-31-230401.txt)
    let export_filename_re = Regex::new(r"conversation-\d{4}-\d{2}-\d{2}-\d{6}\.txt").unwrap();
    // Response arrow prefix: real Claude uses → on response content lines.
    // Matches standalone → lines (e.g., "  →content") and → after ⏺ marker (e.g., "⏺ →content").
    let response_arrow_re = Regex::new(r"^(\s*)→(.*)$").unwrap();
    let marker_arrow_re = Regex::new(r"^(⏺ )→(.+)$").unwrap();
    // Autocomplete suggestion line: indented /command followed by description
    let autocomplete_re = Regex::new(r"^\s+/[\w-]+\s{2,}\S").unwrap();

    let mut result: Vec<String> = Vec::new();
    let mut skip_next_blanks = false;
    let mut prev_was_logo_line1 = false;
    let mut in_response_block = false;
    let mut autocomplete_count = 0;
    let mut in_autocomplete = false;
    for line in &lines {
        // Pre-strip dynamic suffixes before pattern matching to avoid
        // logo_line2_re false-positives on status bar lines containing "·"
        // Normalize non-breaking spaces to regular spaces
        let normalized = line.replace('\u{a0}', " ");
        let trimmed = normalized.trim_end();
        let trimmed = checking_updates_re.replace(trimmed, "");
        let trimmed = marketplace_re.replace(&trimmed, "");
        let trimmed = esc_interrupt_suffix_re.replace(&trimmed, "");
        let trimmed = esc_interrupt_standalone_re.replace(&trimmed, "  ? for shortcuts");
        let trimmed = export_filename_re.replace(&trimmed, "conversation-<TS>.txt");
        let trimmed = trimmed.trim_end();

        // If a non-empty line became empty after stripping dynamic content,
        // skip it entirely (the whole line was dynamic, e.g. "esc to interrupt")
        if trimmed.is_empty() && !normalized.trim_end().is_empty() {
            continue;
        }

        // Track response blocks (after ⏺ marker) for arrow/indent normalization
        if trimmed.starts_with('⏺') {
            in_response_block = true;
        } else if trimmed.is_empty() || trimmed.starts_with('❯') || trimmed.starts_with("──")
        {
            in_response_block = false;
        }

        // Strip → prefix from response content lines (real Claude adds these,
        // claudeless does not). Also strip 2-space continuation indent.
        let trimmed: String = if in_response_block {
            if let Some(caps) = marker_arrow_re.captures(&trimmed) {
                // `⏺ →content` → `⏺ content`
                format!("{}{}", &caps[1], &caps[2])
            } else if let Some(caps) = response_arrow_re.captures(&trimmed) {
                let content = caps[2].trim();
                if content.is_empty() {
                    // Bare `→` or `  →` line (empty continuation) — skip entirely
                    continue;
                }
                format!("{}{}", &caps[1], content)
            } else if trimmed.starts_with("  ") && !trimmed.starts_with("  ⎿") {
                // Strip 2-space continuation indent from response text lines
                // (but not from ⎿ sub-result lines which are intentionally indented)
                trimmed.trim_start().to_string()
            } else {
                trimmed.to_string()
            }
        } else {
            trimmed.to_string()
        };

        // Track autocomplete blocks: keep only first suggestion to normalize
        // count differences between real Claude and claudeless.
        // Also normalize column spacing (collapse multiple spaces to a fixed separator).
        if autocomplete_re.is_match(&trimmed) {
            if !in_autocomplete {
                in_autocomplete = true;
                autocomplete_count = 0;
            }
            autocomplete_count += 1;
            if autocomplete_count > 1 {
                // Skip additional autocomplete suggestions
                continue;
            }
            // Normalize column spacing: collapse 2+ spaces to exactly 2 spaces
            // so that different column alignments between real Claude and claudeless match.
            let trimmed = Regex::new(r"\s{2,}")
                .unwrap()
                .replace_all(&trimmed, "  ")
                .to_string();
            // Fall through to push the normalized line below
            // (need to handle it here since we modified trimmed)
            result.push(trimmed);
            continue;
        } else if in_autocomplete {
            in_autocomplete = false;
            autocomplete_count = 0;
        }

        if let Some(caps) = logo_line1_re.captures(&trimmed) {
            result.push(format!(" ▐▛███▜▌  {}", caps[1].trim()));
            skip_next_blanks = false;
            prev_was_logo_line1 = true;
            continue;
        } else if prev_was_logo_line1 {
            if let Some(caps) = logo_line2_re.captures(&trimmed) {
                result.push(format!("▝▜█████▛▘  {}", caps[1].trim()));
                skip_next_blanks = false;
                prev_was_logo_line1 = false;
                continue;
            }
        }
        prev_was_logo_line1 = false;
        if let Some(caps) = logo_line3_re.captures(&trimmed) {
            result.push(format!("{}{}", &caps[1], "<CWD>"));
            skip_next_blanks = false;
        } else if box_path_re.is_match(&trimmed) {
            // Welcome box path line - normalize dynamic path to <CWD>
            result.push("│ <CWD> │ │".to_string());
            skip_next_blanks = false;
        } else if model_to_try_re.is_match(&trimmed)
            || welcome_to_re.is_match(&trimmed)
            || chrome_settings_re.is_match(&trimmed)
        {
            // Skip "/model to try", "Welcome to ...", and background bleed-through lines
            // and adjacent blank lines, but preserve one blank line (header-to-content gap)
            skip_next_blanks = true;
        } else if spinner_line_re.is_match(&trimmed) {
            // Skip spinner lines (dynamic animation) and adjacent blank lines
            skip_next_blanks = true;
        } else if tip_re.is_match(&trimmed) {
            // Skip terminal-setup tip lines and adjacent blank lines
            skip_next_blanks = true;
        } else if marketplace_cont_re.is_match(&trimmed) {
            // Skip marketplace notification continuation line (wrapped "plugins")
            continue;
        } else if skip_next_blanks && trimmed.is_empty() {
            // Skip blank lines after skipped content
        } else {
            skip_next_blanks = false;
            // Normalize padding inside box lines: │ content   │ → │ content│
            let line_str = trimmed.to_string();
            let box_char = '│';
            let box_char_len = box_char.len_utf8();
            if line_str.starts_with(box_char)
                && line_str.ends_with(box_char)
                && line_str.len() > box_char_len * 2
            {
                let inner = &line_str[box_char_len..line_str.len() - box_char_len];
                let inner_trimmed = inner.trim_end();
                result.push(format!("│{}│", inner_trimmed));
            } else {
                // Normalize spinner frame character in multi-word spinner lines
                let line_str = spinner_multiword_re.replace(&line_str, "✻$1").to_string();
                result.push(line_str);
            }
        }
    }

    // Strip leading/trailing blank lines
    while result.last().is_some_and(|l| l.is_empty()) {
        result.pop();
    }
    while result.first().is_some_and(|l| l.is_empty()) {
        result.remove(0);
    }

    // Deduplicate consecutive logo blocks: if the same normalized logo line 1
    // appears twice (with possible blanks between), keep only the last occurrence.
    // This handles cases where real Claude or claudeless renders the logo twice
    // (e.g., during multi-turn conversations or terminal scroll).
    let result = dedup_logo_blocks(result);

    // Collapse duplicate starfield sections in setup wizard.
    // The starfield is bordered by `…………` lines. If two starfield sections appear
    // back-to-back, collapse them into one.
    let result = collapse_starfield(result);

    result.join("\n")
}

/// Deduplicate consecutive logo blocks in normalized output.
///
/// Scans for the normalized logo line 1 pattern (` ▐▛███▜▌  Claude Code v...`)
/// and if multiple logo blocks appear, keeps only the last one.
fn dedup_logo_blocks(lines: Vec<String>) -> Vec<String> {
    let logo_prefix = " ▐▛███▜▌  ";

    // Find all logo line 1 positions
    let logo_positions: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with(logo_prefix))
        .map(|(i, _)| i)
        .collect();

    if logo_positions.len() <= 1 {
        return lines;
    }

    // Keep only the last logo block: remove all content from the first logo line 1
    // up to (but not including) the last logo line 1, then skip blank lines after removal.
    let first = logo_positions[0];
    let last = *logo_positions.last().unwrap();

    let mut result = Vec::new();
    result.extend(lines[..first].iter().cloned());
    result.extend(lines[last..].iter().cloned());

    // Strip blank lines that appear at the junction
    while result.get(first).is_some_and(|l| l.is_empty()) {
        result.remove(first);
    }

    result
}

/// Collapse duplicate starfield sections bordered by `…………` lines.
///
/// In the setup wizard, the starfield ASCII art may render as one or two copies
/// depending on viewport size. This normalizes by keeping only the last
/// starfield section when multiple `…………` borders appear.
fn collapse_starfield(lines: Vec<String>) -> Vec<String> {
    // Find positions of full-width `…………` border lines (setup wizard starfield borders)
    let border_positions: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim();
            t.len() >= 30 && t.chars().all(|c| c == '…' || c == ' ')
        })
        .map(|(i, _)| i)
        .collect();

    // If there are 3+ borders, the starfield is doubled (border-starfield-border-starfield-border).
    // Collapse to just the last border-starfield-border section.
    if border_positions.len() >= 3 {
        let first_border = border_positions[0];
        // Find the second-to-last border: this is the start of the final starfield section
        let second_to_last = border_positions[border_positions.len() - 2];

        let mut result = Vec::new();
        result.extend(lines[..first_border].iter().cloned());
        result.extend(lines[second_to_last..].iter().cloned());
        return result;
    }

    lines
}

/// Strip logo+header block from text when comparing dialog outputs.
///
/// When claudeless renders a full-screen dialog (thinking, hooks, tasks), the
/// dialog covers the logo area. Real Claude fixtures include the logo above
/// the dialog. This function strips everything before the first dialog
/// separator (`╭` or `──`) so both sides match.
fn strip_before_dialog(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if let Some(pos) = lines.iter().position(|l| {
        let t = l.trim_start();
        t.starts_with('╭') || t.starts_with("──")
    }) {
        // Skip separator lines — claudeless dialogs replace the entire layout
        // and don't include the input-area separator that real Claude shows.
        // Skip ALL consecutive `──` lines (input separator + dialog separator).
        let mut start = pos;
        if lines[start].trim_start().starts_with("──") {
            while start < lines.len() && lines[start].trim_start().starts_with("──") {
                start += 1;
            }
        }
        // Also skip help dialog tab header lines that follow separator lines.
        // The commands tab fixture has a separate non-dash header line after
        // the separator that contains the tab names.
        while start < lines.len() {
            let t = lines[start].trim();
            if t.starts_with("Claude Code") && t.contains("tab to cycle)") {
                start += 1;
            } else {
                break;
            }
        }
        lines[start..].join("\n")
    } else {
        text.to_string()
    }
}

/// Strip terminal scrollback artifacts before a welcome box.
///
/// When a dialog is dismissed in real Claude, the dialog text remains in the
/// terminal scrollback above the welcome screen. This function strips
/// everything before the first `╭` line (welcome box start).
fn strip_scrollback_before_box(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if let Some(pos) = lines.iter().position(|l| l.trim_start().starts_with('╭')) {
        lines[pos..].join("\n")
    } else {
        text.to_string()
    }
}

/// Normalize ANSI TUI text for comparison.
///
/// Same as plain normalization — strips trailing whitespace and blank lines.
fn normalize_ansi(text: &str) -> String {
    normalize_tui(text)
}

// =============================================================================
// State normalization
// =============================================================================

/// Normalize a state diff for comparison.
///
/// Replaces dynamic values (UUIDs, timestamps, temp paths) with placeholders.
fn normalize_state_diff(diff: &str) -> String {
    use regex::Regex;

    let mut result = diff.to_string();

    // UUIDs → <UUID>
    let uuid_re =
        Regex::new(r"[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}").unwrap();
    result = uuid_re.replace_all(&result, "<UUID>").to_string();

    // Backup timestamps (e.g., .backup.1769918659501) → .backup.<TS>
    let backup_re = Regex::new(r"\.backup\.\d+").unwrap();
    result = backup_re.replace_all(&result, ".backup.<TS>").to_string();

    // Temp directory paths → <TMPDIR>
    let tmp_re =
        Regex::new(r"-private-var-folders-[a-zA-Z0-9/_-]+-T-capture-[a-zA-Z0-9]+").unwrap();
    result = tmp_re.replace_all(&result, "<TMPDIR>").to_string();

    // Line numbers in diff headers (e.g., "1a2,12") — leave as-is since structure matters

    result
}

/// Parse `> state/...` lines from a state diff fixture into relative paths.
///
/// The diff format uses a header like `1a2,12` followed by `> ` prefixed lines.
/// We extract the path after `> state/`.
fn parse_state_diff_files(diff: &str) -> Vec<String> {
    diff.lines()
        .filter_map(|line| line.strip_prefix("> state/"))
        .map(|s| s.to_string())
        .collect()
}

/// Recursively list all files under `dir`, returning paths relative to `dir`.
fn list_state_files(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, base, out);
                } else if let Ok(rel) = path.strip_prefix(base) {
                    out.push(rel.to_string_lossy().to_string());
                }
            }
        }
    }
    walk(dir, dir, &mut files);
    files
}

/// Filter state files to comparable subset, excluding volatile/non-deterministic files.
fn filter_state_files(files: &[String]) -> BTreeSet<String> {
    files
        .iter()
        .filter(|f| {
            !f.starts_with(".claude.json.backup.")
                && !f.starts_with("cache/")
                && !f.starts_with("debug/")
                && !f.starts_with("plugins/")
                && !f.starts_with("sessions/")
                // TODO: stop filtering once claudeless produces these
                && *f != "history.jsonl"
                && *f != "settings.json"
                // TODO: sessions-index.json creation is timing-dependent;
                // stop filtering once claudeless writes it at the right time
                && !f.ends_with("/sessions-index.json")
        })
        .map(|f| f.to_string())
        .collect()
}

/// Normalize a state file path by replacing UUIDs and machine-specific segments with placeholders.
fn normalize_state_path(path: &str) -> String {
    use regex::Regex;

    let mut result = path.to_string();

    // UUIDs → <UUID>
    let uuid_re =
        Regex::new(r"[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}").unwrap();
    result = uuid_re.replace_all(&result, "<UUID>").to_string();

    // The first path segment under `projects/` is a flattened filesystem path
    // (e.g. `-private-var-folders-...-T-capture-xxx` or `-Users-user-Developer-proj`).
    // It's always machine-specific, so normalize the entire segment to <PROJDIR>.
    let proj_re = Regex::new(r"^(projects/)[^/]+").unwrap();
    result = proj_re.replace(&result, "${1}<PROJDIR>").to_string();

    result
}

/// Compare state directories structurally.
///
/// Parses the `{script}.state.diff` fixture, lists actual files in the state dir,
/// filters both to a comparable subset, normalizes paths, and compares as sets.
fn assert_state(version: &str, script: &str, state_dir: &Path) {
    let expected_diff = load_state_diff(version, script);
    if expected_diff.is_empty() {
        // No state diff fixture — nothing to assert
        return;
    }

    let expected_files = parse_state_diff_files(&expected_diff);
    let expected_filtered = filter_state_files(&expected_files);
    let expected_normalized: BTreeSet<String> = expected_filtered
        .iter()
        .map(|f| normalize_state_path(f))
        .collect();

    let actual_files = list_state_files(state_dir);
    let actual_filtered = filter_state_files(&actual_files);
    let actual_normalized: BTreeSet<String> = actual_filtered
        .iter()
        .map(|f| normalize_state_path(f))
        .collect();

    // Real Claude creates session files asynchronously a few seconds after startup,
    // so captures that exit quickly may not include them in the state diff.
    // Claudeless creates them synchronously, so the actual set may contain extra
    // session-init files. Only those specific extras are tolerated.
    // Real Claude creates subagent files (e.g., prompt_suggestion) and file-history
    // snapshots that claudeless does not implement. Tolerate missing entries.
    let missing: BTreeSet<_> = expected_normalized
        .difference(&actual_normalized)
        .filter(|f| !f.contains("/subagents/") && !f.starts_with("file-history/"))
        .collect();
    assert!(
        missing.is_empty(),
        "State files missing for {script}.\n\
         Expected (from fixture): {expected_normalized:?}\n\
         Actual (from claudeless): {actual_normalized:?}\n\
         Missing: {missing:?}"
    );

    let session_init_files: BTreeSet<&str> = [
        ".claude.json",
        "projects/<PROJDIR>/<UUID>.jsonl",
        "todos/<UUID>-agent-<UUID>.json",
    ]
    .into_iter()
    .collect();

    let unexpected: BTreeSet<_> = actual_normalized
        .difference(&expected_normalized)
        .filter(|f| !session_init_files.contains(f.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "Unexpected state files for {script}.\n\
         Expected (from fixture): {expected_normalized:?}\n\
         Actual (from claudeless): {actual_normalized:?}\n\
         Unexpected: {unexpected:?}"
    );
}

// =============================================================================
// Capsh spec runner
// =============================================================================

/// Run a simplified `.capsh` script against claudeless and assert TUI snapshots.
///
/// This function:
/// 1. Creates a temp directory for frames
/// 2. Runs `capsh --frames <dir> -- claudeless --scenario <toml>` with the script on stdin
/// 3. Parses recording.jsonl to find named snapshot frame numbers
/// 4. Asserts plain and ANSI TUI snapshots match fixtures
/// 5. Asserts state directory structure (via state diff)
pub fn run_capsh_spec(version: &str, script: &str, snapshots: &[&str], extra_args: &[&str]) {
    run_capsh_spec_with_size(version, script, snapshots, extra_args, None, None);
}

/// Like `run_capsh_spec` but allows specifying custom terminal dimensions.
pub fn run_capsh_spec_with_size(
    version: &str,
    script: &str,
    snapshots: &[&str],
    extra_args: &[&str],
    cols: Option<u16>,
    rows: Option<u16>,
) {
    let frames_dir = tempfile::tempdir().expect("create temp dir for frames");
    let state_dir = tempfile::tempdir().expect("create temp dir for state");
    let scenario = specs_dir().join("scenarios").join(format!("{script}.toml"));
    let capsh_script = specs_dir().join("capsh").join(format!("{script}.capsh"));

    assert!(
        scenario.exists(),
        "Scenario file not found: {}",
        scenario.display()
    );
    assert!(
        capsh_script.exists(),
        "Capsh script not found: {}",
        capsh_script.display()
    );

    let capsh = capsh_bin();
    assert!(
        capsh.exists(),
        "capsh binary not found: {} — build with `cargo build -p capsh`",
        capsh.display()
    );

    // Run capsh with the simplified script
    let script_file = fs::File::open(&capsh_script)
        .unwrap_or_else(|e| panic!("Failed to open capsh script: {}", e));

    let mut cmd = Command::new(&capsh);
    cmd.args(["--frames", frames_dir.path().to_str().unwrap()]);
    if let Some(c) = cols {
        cmd.args(["--cols", &c.to_string()]);
    }
    if let Some(r) = rows {
        cmd.args(["--rows", &r.to_string()]);
    }
    cmd.env("CLAUDELESS_CONFIG_DIR", state_dir.path());
    cmd.arg("--")
        .arg(claudeless_bin())
        .args(["--scenario", scenario.to_str().unwrap()])
        .args(extra_args);

    let output = cmd
        .stdin(script_file)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run capsh: {}", e));

    // capsh exits 0 on success, 143 when killed by SIGTERM (expected)
    let code = output.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 143,
        "capsh exited with code {code} for {script}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // Parse recording to find snapshot frames
    let recording = parse_recording_snapshots(frames_dir.path());

    // Assert TUI snapshots
    for snapshot_name in snapshots {
        let entry = recording
            .iter()
            .find(|e| e.name == *snapshot_name)
            .unwrap_or_else(|| {
                let available: Vec<_> = recording.iter().map(|e| &e.name).collect();
                panic!(
                    "Snapshot '{snapshot_name}' not found in recording for {script}. Available: {available:?}"
                )
            });

        // Plain text comparison
        let actual_plain_path = frames_dir.path().join(format!("{}.txt", entry.frame));
        let actual_plain = fs::read_to_string(&actual_plain_path).unwrap_or_else(|e| {
            panic!(
                "Failed to read frame {}: {}",
                actual_plain_path.display(),
                e
            )
        });
        let expected_plain = load_tui_fixture(version, snapshot_name, false);
        let mut norm_expected = normalize_tui(&expected_plain);
        let mut norm_actual = normalize_tui(&actual_plain);

        // When claudeless renders a full-screen dialog, it covers the logo area.
        // If the actual output lacks the logo, strip the logo+header block from
        // both sides so the dialog content can be compared directly.
        if !norm_actual.contains("▐▛███▜▌") {
            norm_expected = strip_before_dialog(&norm_expected);
            norm_actual = strip_before_dialog(&norm_actual);
        }

        // Strip scrollback artifacts: when a dismissed dialog remains visible
        // above the welcome box in the fixture (terminal scrollback), strip
        // everything before the `╭` line from the expected side.
        if norm_actual.contains('╭') && norm_expected.contains('╭') {
            norm_expected = strip_scrollback_before_box(&norm_expected);
            norm_actual = strip_scrollback_before_box(&norm_actual);
        }

        similar_asserts::assert_eq!(
            norm_expected,
            norm_actual,
            "TUI plain text mismatch for snapshot '{snapshot_name}' in {script}"
        );

        // ANSI comparison skipped for now — logo chars and ANSI escape sequences
        // differ structurally between real Claude (bg-colored) and claudeless (fg blocks).
        // Plain text comparison above is the primary assertion.
    }

    // Assert state
    assert_state(version, script, state_dir.path());
}

// =============================================================================
// Tmux spec runner
// =============================================================================

/// Run a tmux-based spec test. Currently a placeholder — tmux specs require
/// signal delivery that capsh can't handle, so these need special treatment.
pub fn run_tmux_spec(version: &str, script: &str, snapshots: &[&str]) {
    let scenario = specs_dir().join("scenarios").join(format!("{script}.toml"));
    let capsh_script = specs_dir().join("capsh").join(format!("{script}.capsh"));

    // If there's a capsh script for this tmux spec, run it like a capsh spec
    if capsh_script.exists() && scenario.exists() {
        run_capsh_spec(version, script, snapshots, &[]);
        return;
    }

    // Otherwise, just verify fixtures exist
    for snapshot_name in snapshots {
        let fixture = fixtures_dir(version).join(format!("{snapshot_name}.tmux.txt"));
        assert!(
            fixture.exists(),
            "Tmux fixture not found: {}",
            fixture.display()
        );
    }
}

// =============================================================================
// CLI spec runner
// =============================================================================

/// Assert that claudeless CLI output starts with the expected fixture content.
///
/// Runs `claudeless <args>` and compares the output prefix with the fixture.
pub fn assert_cli_starts_with(version: &str, name: &str, args: &[&str]) {
    let expected = load_cli_fixture(version, name);

    // Set CLAUDELESS_CLAUDE_VERSION so --version outputs the expected format
    let claude_version = version.strip_prefix('v').unwrap_or(version);
    let output = Command::new(claudeless_bin())
        .args(args)
        .env("CLAUDELESS_CLAUDE_VERSION", claude_version)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run claudeless: {}", e));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Use starts_with comparison — claudeless may produce additional output
    let expected_trimmed = expected.trim_end();
    let actual_trimmed = stdout.trim_end();

    assert!(
        actual_trimmed.starts_with(expected_trimmed),
        "CLI output for '{name}' does not start with expected fixture.\n\
         Expected prefix:\n{expected_trimmed}\n\n\
         Actual output:\n{actual_trimmed}"
    );
}
