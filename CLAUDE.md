# AI PR Review

Two-pass AI code review GitHub Action, written in Rust. Also ships an MCP server for editor-based review workflows.

## Where to look

- **Specs**: `docs/specs/` — one `requirements.md` per feature, acceptance criteria in WHEN/SHALL format
- **Testing**: `docs/specs/testing.md` — structure, naming conventions, how to verify spec coverage
- **Config schema**: `docs/specs/config-loader/requirements.md`
- **Key behaviours** (SHA storage, incremental review, `@bot` commands): `docs/specs/two-pass-review/requirements.md`
- **Workspace layout**: two binaries (`ai-pr-action`, `ai-pr-mcp`) + shared `core` crate under `crates/`
- **Release**: tag `v*` → CI compiles Linux binary → committed to `bin/` → consumers use `uses: your-org/ai-pr-review@v1`

## AI tool conventions

When working through a feature implementation, create `.ai-reviews/<branch>-tasks.json` to track progress. This file is git-ignored and is only for AI tool use — it is not produced by the application itself.

Schema:
```json
[
  {
    "id": "1",
    "spec": "config-loader",
    "requirement": "Requirement 1",
    "criterion": 2,
    "description": "Missing config returns defaults",
    "status": "pending"
  }
]
```

`status` is one of `"pending"`, `"in_progress"`, `"done"`, or `"skipped"`. Update it as you work through acceptance criteria so you don't re-read the whole plan on each step.

## Running tests

```bash
cargo test          # all tests
cargo test -p core  # single crate
```
