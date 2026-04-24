# Contributing to Nabi

Thank you for your interest in Nabi. Before opening a pull request, please read this document in full.

## Scope of contributions

Nabi follows a maintainer-led model. The core runtime is developed exclusively by the maintainer team to preserve architectural cohesion. External contributions are welcomed and encouraged in specific areas.

### Maintainer-only crates

The following crates are not open for external code contributions. Bug reports, issue discussions, and design feedback are still welcome.

- `nabi-core`
- `nabi-io`
- `nabi-runtime`
- `nabi-orchestration`
- `nabi-macros`

### Community-contributable areas

Pull requests are welcome for:

- `nabi-compat` — Tokio API surface mapping
- `nabi-net`, `nabi-fs`, `nabi-tls` — platform backends and features
- `nabi-lens`, `nabi-scope` — observability emission and tooling
- `nabi-test` — test utilities and mocks
- `nabi` — facade re-exports
- Documentation, examples, benchmarks, integration tests
- Bug fixes in any crate (discuss in an issue first for non-trivial changes)

If you are unsure whether your change falls within the contributable scope, open an issue before writing code.

### Issues

GitHub issues are reserved for community-reported bugs and feature requests. Internal roadmap work is tracked privately and is submitted directly as pull requests without a linked GitHub issue. Do not open GitHub issues for internal planning or status tracking.

## Commit message convention

Nabi follows [Conventional Commits](https://www.conventionalcommits.org/) with project-specific refinements.

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type

One of the following, no other values are accepted:

- `feat` — a new feature
- `fix` — a bug fix
- `perf` — a performance improvement without behavior change
- `refactor` — code change that neither adds a feature nor fixes a bug
- `docs` — documentation only
- `test` — adding or updating tests
- `build` — build system or dependency changes (`Cargo.toml`, `rust-toolchain.toml`)
- `ci` — CI configuration changes (`.github/workflows/`, CI scripts)
- `chore` — maintenance tasks that do not fit other types (version bumps, releases)
- `revert` — revert of a prior commit (see [Reverting commits](#reverting-commits))

### Scope

The scope identifies the primary crate affected, using the crate name with the `nabi-` prefix stripped.

**Allowed scopes:**

`core`, `io`, `net`, `fs`, `tls`, `runtime`, `orchestration`, `compat`, `macros`, `lens`, `scope`, `test`, `facade`, `workspace`, `deps`

The `deps` scope is reserved for Dependabot and manual dependency bumps. All other scopes correspond to crates in the workspace.

Sub-module scopes are permitted when the change is localized to a specific module within a crate:

```
feat(runtime/scheduler): add work-stealing deque
fix(io/uring): handle EINVAL on buf_ring setup
refactor(orchestration/advisor): simplify guard composition
```

Sub-module names must be lowercase, start with a letter, and use only `[a-z0-9_]`. The parent scope must be a valid crate scope from the list above; `deps` and `workspace` do not take sub-modules.

**Multi-crate changes:** choose the dominant crate for the scope and describe the other affected crates in the body.

**Workspace-wide changes** (rare, e.g., global rename, lint policy change) use `workspace`:

```
refactor(workspace): rename Conductor to Regent across all crates
```

### Subject

- **50 characters maximum.** Enforced by CI as a hard failure.
- **Lowercase.** No capital letter at the start.
- **No trailing period.**
- **Imperative mood** — "add", not "added" or "adds".
- **Specific.** State what the change does, not a vague label.

**Bad subjects** (rejected during review):

```
feat(runtime): add scheduler
fix(io): fix bug
refactor(orchestration): cleanup
chore: update deps
```

**Good subjects:**

```
feat(runtime): add work-stealing scheduler with local queues
fix(io/uring): handle partial reads in multishot recv
refactor(orchestration): split guard composition from advisor
ci(deps): bump actions/checkout from 4 to 5
```

### Body

The body is optional for trivial changes (typos, comment fixes, formatting) and strongly recommended for all logic changes. When provided, use the **Problem / Solution** format:

```
feat(runtime): add work-stealing scheduler with local queues

Problem: the single-threaded scheduler bottlenecks on multi-core
systems; tasks queued on one worker cannot be picked up by idle
workers on other cores.

Solution: introduce per-worker local deques backed by crossbeam-deque,
with stealing from other workers when the local queue is empty. Worker
parking integrates with the reactor to avoid spinning.
```

Wrap body lines at 72 characters.

### Footer

**Breaking changes** are marked in **both** places:

1. `!` after the type/scope in the subject line.
2. `BREAKING CHANGE:` footer describing the change and migration path.

```
feat(runtime)!: replace spawn with run_* entry points

Problem: spawn implies a single scheduler mode, incompatible with
Nabi's dual-scheduler design.

Solution: introduce run_affine, run_stealing, and run_blocking as
explicit entry points; users must choose the scheduler mode.

BREAKING CHANGE: spawn() is removed. Replace with run_affine for
thread-per-core tasks, run_stealing for work-stealing tasks, or
run_blocking for blocking operations.
```

**Issue references** use `Refs:` or `Fixes:` only, and only when the PR resolves a GitHub issue (community-reported). Internal work does not reference GitHub issues. Linear identifiers must never appear in commit messages. `Closes:` is forbidden in commits (reserved for pull request descriptions targeting GitHub issues).

```
Refs: #42
Fixes: #87
```

If no related issue exists, omit the footer entirely.

**Co-authors** are declared with the standard Git trailer. Co-author trailers must refer to human contributors only.

```
Co-authored-by: Name <email@example.com>
```

### Complete example

```
feat(orchestration/advisor)!: enforce guard composition order

Problem: the previous advisor API allowed policies to be composed
in arbitrary order, producing unexpected interactions between
retry, timeout, and circuit breaker layers.

Solution: fix the composition order to acquire -> timeout -> retry
-> breaker -> execute. The guard() builder now emits a type error
if layers are added in the wrong order.

BREAKING CHANGE: Advisor::decide() is removed. Use guard() to
compose policies in the canonical order.

Refs: #128
```

## Branch naming

Branch names follow `<type>/<short-description>`, 35 characters maximum. Issue numbers do not appear in branch names.

```
feat/work-stealing-scheduler
fix/uring-partial-reads
refactor/advisor-composition
docs/runtime-overview
```

Allowed types are the same as the commit type list above.

## Pull requests

### Title

Pull request titles follow the same rules as commit subjects: Conventional Commits format with a 50-character subject limit, enforced by CI. On squash merge, the PR title becomes the commit title with `(#NN)` appended automatically by GitHub.

### Body

The PR body uses the repository's template with `Problem`, `Solution`, and optionally `Closes #NN` for GitHub-tracked issues. Do not reference internal issue identifiers.

### Size

Soft limits:

- 20 files changed
- +400 lines added

Larger changes should be split when feasible. Exceptions require justification in the PR description.

### Merge strategy

Squash merge is the only accepted merge strategy. The maintainer merges pull requests; external contributors do not merge their own PRs.

On merge, the squash commit title mirrors the PR title (with `(#NN)` appended automatically). **The squash commit body is left empty** — all discussion, rationale, and linked issues remain accessible via the PR itself, which is the permanent record.

## Labels

The repository uses 16 labels. Every label is either auto-applied by CI or applied manually by the PR author. Maintainers do not apply labels during review.

### Auto-applied

**`crate:*`** — applied by `.github/labeler.yml` based on changed file paths. 14 labels, one per crate plus `crate:workspace` for root configuration. Do not apply manually.

- `crate:core`, `crate:io`, `crate:net`, `crate:fs`, `crate:tls`
- `crate:runtime`, `crate:orchestration`, `crate:compat`, `crate:macros`
- `crate:lens`, `crate:scope`, `crate:test`, `crate:facade`
- `crate:workspace` — root config, CI, `.cargo/`, `.github/`

**`safety:unsafe`** — applied when the diff contains the `unsafe` keyword. Triggers mandatory review focus on the unsafe block. Do not apply manually.

### Manual

**`needs-adr`** — applied by the PR author when the change introduces a design decision that requires an Architecture Decision Record.

Apply `needs-adr` when the pull request:

- Introduces or modifies a root principle (arena-centric ownership, dual-scheduler separation, I/O completion-first, etc.)
- Selects among two or more valid options with non-trivial rationale
- Changes an external boundary: public API signature, wire format, FFI boundary, MSRV, edition, or toolchain

Do **not** apply `needs-adr` for:

- Implementation of an already-decided principle
- Refactors, bug fixes, renames
- Dependency version bumps
- Test or documentation additions
- CI tuning

When in doubt, apply the label. ADR drafting happens in parallel with the pull request and must be completed before merge.

### ADR visibility

ADRs are maintained internally and are not published. **Do not** link to the ADR, reference its identifier, or quote its content in the pull request title, description, commit message, or comments. The `needs-adr` label alone communicates that an ADR exists.

A single ADR may span multiple pull requests; re-apply the label to each.

## Reverting commits

Revert commits follow the Conventional Commits format. The default subject generated by `git revert` must be rewritten to conform.

Use the `revert` type with the scope of the crate being reverted:

```
revert(runtime): work-stealing scheduler

Problem: the work-stealing scheduler introduced in abc123d caused
deadlocks under high contention; root cause analysis is pending.

Solution: revert to restore stability while the issue is
investigated.

Reverts: abc123def456
Refs: #142
```

The `Reverts:` footer records the original commit hash. If the revert is tracked by a GitHub issue, include `Refs:` or `Fixes:` as usual. Internal issue identifiers do not appear.

## License

Nabi is dual-licensed under the Apache License, Version 2.0 and the MIT License.

By submitting a pull request, you agree that your contributions will be dual-licensed under the same terms, without any additional conditions.

```
SPDX-License-Identifier: MIT OR Apache-2.0
```
