# Tag and release a new version, then publish to crates.io.
#
# Validates prerequisites, bumps version if needed, waits for CI, creates tag,
# and publishes to crates.io. Updates homebrew tap at end.
#
# Prerequisites:
#   - Must be on 'main' branch with clean working tree
#   - Must be logged into crates.io (cargo login)
#
# Examples:
#   oj run release

command "release" {
  args = ""
  run  = { agent = "release" }
}

agent "release" {
  run     = "claude --model opus --dangerously-skip-permissions"
  on_idle = { action = "escalate" }
  on_dead = { action = "escalate" }

  prime = [
    "echo '## Current State'",
    "git branch --show-current",
    "git status --short",
    "echo '## Version Info'",
    "grep '^version = ' Cargo.toml | head -1",
    "echo 'Latest tag:' $(git tag --sort=-v:refname | grep '^v' | head -1 || echo 'none')",
    "echo '## Commits Since Last Tag'",
    "git log $(git describe --tags --abbrev=0)..HEAD --oneline",
    "echo '## CHANGELOG.md (head)'",
    "head -30 CHANGELOG.md 2>/dev/null || echo 'No CHANGELOG.md'",
  ]

  prompt = <<-PROMPT
    Release claudeless to crates.io and update the homebrew tap.

    ## Steps

    1. **Validate prerequisites**
       - Confirm on 'main' branch
       - Confirm working tree is clean
       - Verify crates.io auth: `cargo publish --dry-run -p claudeless`

    2. **Run checks**: `make check`

    3. **Bump version if needed**
       - Compare Cargo.toml version with latest git tag
       - If version <= latest tag, bump patch version in Cargo.toml

    4. **Update CHANGELOG.md**
       - Add a new section for the version being released
       - Summarize changes since the last tag: `git log $(git describe --tags --abbrev=0)..HEAD --oneline`
       - Group by type: Features, Fixes, Chores
       - Commit: `git commit -am "chore: release vX.Y.Z"`

    5. **Push to remotes**
       - `git push origin main`
       - `git push github main`

    6. **Wait for CI**
       - Use `gh api repos/{owner}/{repo}/commits/$(git rev-parse HEAD)/check-runs` to poll
       - Wait until all checks pass (poll every 10s, timeout after 10 min)

    7. **Create and push tag**
       - `git tag -a vX.Y.Z -m "Release vX.Y.Z"`
       - `git push origin vX.Y.Z`
       - `git push github vX.Y.Z`

    8. **Wait for release build**
       - Use `gh run list --branch vX.Y.Z` to poll for workflow completion

    9. **Publish to crates.io**: `cargo publish -p claudeless`

    10. **Update homebrew tap**
       - cd to ../homebrew-tap
       - Download tarball and compute SHA256:
         `curl -sL https://github.com/kestred/claudeless/archive/refs/tags/vX.Y.Z.tar.gz | shasum -a 256`
       - Update Formula/claudeless.rb with new url, version, sha256
       - Commit and push

    Report the release URL and crates.io URL when done.
  PROMPT
}
