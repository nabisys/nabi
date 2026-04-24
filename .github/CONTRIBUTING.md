# Contributing to Nabi

Thank you for your interest in Nabi. Before opening a pull request, please read this document in full.

## Scope of contributions

Nabi follows a maintainer-led model, similar to Tokio. The core runtime is developed exclusively by the maintainer team to preserve architectural cohesion. External contributions are welcomed and encouraged in specific areas.

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

`core`, `io`, `net`, `fs`, `tls`, `runtime`, `orchestration`, `compat`, `macros`, `lens`, `scope`, `test`, `facade`, `workspace`

Sub-module scopes are permitted when the change is localized to a specific module within a crate:

```
feat(runtime/scheduler): add work-stealing deque
fix(io/uring): handle EINVAL on buf_ring setup
refactor(orchestration/advisor): simplify guard composition
```

**Multi-crate changes:** choose the dominant crate for the scope and describe the other affected crates in the body.

**Workspace-wide changes** (rare, e.g., global rename, lint policy change) use `workspace`:

```
refactor(workspace): rename Conductor to Regent across all crates
```

### Subject

- **50 characters recommended**, 72 characters maximum.
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
chore(deps): bump rustls from 0.23.20 to 0.23.21
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

**Issue references** use `Refs:` or `Fixes:` only. Nabi uses Linear for issue tracking; Linear identifiers must **not** appear in commit messages. `Closes:` is forbidden in commits (reserved for pull request descriptions targeting GitHub issues).

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

Pull request titles follow the Conventional Commits format and are **45 characters maximum** (excluding the `(#NN)` suffix that GitHub appends on squash merge).

### Body

The PR body uses the repository's template with `Problem`, `Solution`, and optionally `Closes #NN` for GitHub-tracked issues. Do not reference Linear identifiers.

### Size

Soft limits:

- 20 files changed
- +400 lines added

Larger changes should be split when feasible. Exceptions require justification in the PR description.

### Merge strategy

Squash merge is the only accepted merge strategy. The maintainer merges pull requests; external contributors do not merge their own PRs.

On merge, the squash commit title mirrors the PR title (with `(#NN)` appended automatically). **The squash commit body is left empty** — all discussion, rationale, and linked issues remain accessible via the PR itself, which is the permanent record.

## Reverting commits

Revert commits use Git's default format, not Conventional Commits:

```
Revert "feat(runtime): add work-stealing scheduler"

This reverts commit abc123def456.
```

The subject is generated by `git revert` and is not edited to conform to the Conventional Commits format. Include a brief explanation in the body describing the reason for the revert.

## License

Nabi is dual-licensed under the Apache License, Version 2.0 and the MIT License.

By submitting a pull request, you agree that your contributions will be dual-licensed under the same terms, without any additional conditions.

```
SPDX-License-Identifier: MIT OR Apache-2.0
```
