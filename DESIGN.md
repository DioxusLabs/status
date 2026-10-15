# Dioxus Open-Source Health Dashboard — Design Proposal

Goal: a daily-driver dashboard that reduces the friction of maintaining the DioxusLabs org.
It should answer, in one screen: *what needs my attention today, what's ready to merge, how healthy is the project, and what can I hand to a bot.*

Current scale (live numbers, 2026-09-11): 56 non-archived repos, ~430 open PRs, ~980 open issues.
The bulk is `dioxus` (122 PRs / 651 issues), `blitz` (110 / 76), `taffy` (57 / 83), `docsite` (30 / 93), plus the
Linebender forks (`parley`, `stylo`, `vello`, `peniko`, …) which carry fork PRs and mostly aren't "ours".

---

## 1. Architecture

```
┌──────────────────────┐   cron / webhook    ┌─────────────────────┐
│  GitHub GraphQL+REST │ ──────────────────► │  collector (Rust)   │
│  crates.io / docs.rs │                     │  - fetch + normalize│
│  GH Actions / traffic│                     │  - score PRs        │
└──────────────────────┘                     │  - LLM assessment   │
                                             └─────────┬───────────┘
                                                       │ writes
                                             ┌─────────▼───────────┐
                                             │  SQLite (sqlx)      │
                                             │  + nightly JSON     │
                                             │    snapshots → repo │
                                             └─────────┬───────────┘
                                                       │ server fns
┌──────────────────────┐    Devin API         ┌─────────▼───────────┐
│  Devin sessions      │ ◄─────────────────── │  Dioxus 0.8 alpha   │
│  (playbooks/skills)  │ ───► PR comments     │  fullstack web app  │
└──────────────────────┘                      └─────────────────────┘
```

- **Framework**: Dioxus `0.8.0-alpha.1` fullstack (web + server, SSR + hydrate). Workspace mirrors the docsite
  (tailwind, dark theme, same nav/footer look) so it can later be folded into dioxuslabs.com.
- **Single deployable**: an axum server binary (same Dockerfile/fly.toml pattern as the docsite). The collector
  runs inside the server on a tokio interval (PRs every 5 min, metrics hourly, LLM assessments on head-SHA change)
  and can also be triggered by a GitHub webhook for near-realtime PR updates.
- **Storage**: SQLite via `sqlx` (single file volume on fly). Time-series that GitHub only keeps 14 days of
  (traffic views/clones) get snapshotted daily. A nightly job also commits a compact `data/*.json` snapshot to the
  `status` repo — this matches the original README idea and gives a free, versioned, forkable history.
- **Auth**: read-only pages public; "act" pages (trigger Devin, dismiss, pin) behind GitHub OAuth restricted to
  org members (or a single shared token env var for v1).
- **Credentials**: a GitHub App installed on the org (higher rate limits, `repo`/`admin:repo_hook`; traffic stats
  require push access), `DEVIN_API_KEY`, `ANTHROPIC_API_KEY` (or route LLM work through Devin instead).

## 2. Data sources

| Source | What we pull | Notes |
|---|---|---|
| GitHub GraphQL | all open PRs across org: title, body, author, association (MEMBER/CONTRIBUTOR/FIRST_TIMER), labels, draft, `mergeable`, `reviewDecision`, review threads (resolved?), latest commit `statusCheckRollup`, files/additions/deletions, linked issues, timeline (last maintainer vs contributor activity), milestone | one paginated query per repo; ~50 repos ≈ 60 points/cycle |
| GitHub GraphQL | issues: labels, reactions, comments count, age, assignee, `stateReason`, first-response time | for triage view |
| GitHub REST | releases + asset download counts (`dx` binaries), repo traffic (views/clones/referrers), stars/forks, contributors, Actions workflow runs on `main` (pass rate, duration) | traffic is 14-day window → snapshot |
| crates.io API | per-crate total + recent (90d) downloads, per-version downloads, daily downloads (`/downloads`), reverse-dependency counts, latest version/date | crates: dioxus, dioxus-core, dioxus-cli, dioxus-web, dioxus-desktop, dioxus-fullstack, dioxus-router, dioxus-signals, taffy, blitz-*, dioxus-components, dioxus-sdk, manganis, … (configurable list) |
| docs.rs | build status per crate/version | cheap health signal |
| lib.rs / GitHub dependents | "used by" counts | optional, scrape-y |
| Discord (optional) | member count, help-channel volume | later |
| Devin API | sessions we launched: status, PR opened, cost | for the "bot work" panel |

## 3. Pages / features

### 3.1 Today (home)
The daily-driver view. Sections, each a compact list with counts:
- **Ready to merge** — score ≥ threshold, CI green, approved, no conflicts.
- **Waiting on me** — review requested from me / last comment from contributor / my PRs with changes requested.
- **Quick wins** — small (<50 LOC), green CI, docs/typo/deps.
- **Going stale** — contributor PRs with no maintainer response in > 7/14/30 days (this is the reputation killer).
- **Broken main** — repos whose latest `main` workflow run failed.
- **New this week** — new issues/PRs/contributors, first-time contributors to greet.
- **Bot activity** — Devin sessions running / finished / needing review.
- Health strip: downloads (7d Δ), stars (7d Δ), open PRs/issues (Δ), median PR age, CI pass rate.

### 3.2 Pull requests (the big table)
- Unified across all repos, virtualized table, keyboard navigation (j/k, enter to open, `m` mark).
- **Filters**: repo (multi), author / association (member, contributor, first-timer, bot), labels, draft, CI state,
  review decision, mergeable/conflicts, size bucket, age, last-activity-by (maintainer/contributor/none),
  linked-issue, has tests, touches docs, breaking (label or semver-check), milestone, "unresolved threads".
- **Search**: full-text over title/body/file paths/comments (SQLite FTS5). Query syntax like GitHub's
  (`repo:dioxus is:green author:ealmloff -label:blocked`), plus saved views.
- **Smart sorts**: mergeability score, "attention needed", newest, oldest, last activity, size, staleness, CI first.
- **Row expansion**: LLM summary, score breakdown, checks list, unresolved threads, files touched, related PRs
  (same files), linked issues, action buttons (open on GH, trigger Devin, snooze, pin).
- **Bulk actions**: label, comment template ("thanks, will review"), close stale, kick off Devin on N PRs.
- **Dedup/related**: PRs touching the same files or referencing the same issue get grouped (common in dioxus).

### 3.3 Mergeability score
Two layers, cached per (PR, head SHA, base SHA):

**Deterministic score (0–100)** — transparent, shown as a breakdown:
```
+ CI green (all required checks)            25   (yellow/pending 10, red 0)
+ mergeable (no conflicts)                  15
+ review: approved 15 / none 5 / changes-requested -15
+ unresolved review threads                 -3 each (cap -15)
+ size: xs 10, s 8, m 5, l 2, xl 0          (additions+deletions, files)
+ has tests (touches tests/ or #[test])      8
+ links an issue / has description           5
+ author: maintainer 5, returning 3, first-timer 0 (not penalized, just less prior)
+ recency: activity < 7d 5, > 90d -10
+ touches docs when public API changed       3
- draft                                     -30 (excluded from "ready" lists)
- conflicts with another open PR (same files)  -5
- label:blocked / needs-design / breaking   -10..-20
```

**LLM assessment** (Claude via API, or a Devin session using a review playbook) — runs when head SHA changes:
- 2-sentence summary of intent, risk level (low/med/high), quality (1–5) with reasons, checklist:
  tests added? docs updated? changelog? public API changed? follows repo conventions (e.g. no unwrap in core)?
  what a maintainer should check first, suggested labels, suggested action (merge / request changes / close / needs-rebase).
- Prompt receives: title, body, diff (truncated/summarized for large PRs), CI summary, unresolved review threads,
  repo-specific rubric (`rubrics/dioxus.md`, `rubrics/taffy.md`).
- Output stored as structured JSON; feeds a ±15 adjustment to the deterministic score and the row expansion panel.
- Cost control: only PRs not draft, ≤ 3k changed lines get full diff; big PRs get file-list + description only.

### 3.4 Issues / triage
- Same table machinery for issues: untriaged (no labels), no response, high reactions, duplicates cluster
  (LLM embedding similarity), regressions, good-first-issue candidates, stale-closable.
- "Issue → Devin" action: spin up a session with the issue as prompt + repo skills.

### 3.5 Health & metrics
- **Downloads**: per-crate daily/weekly chart, per-version split (adoption of 0.7 → 0.8), ecosystem
  (crates depending on dioxus), `dx` release asset downloads per platform.
- **Activity**: commits/week per repo, PRs opened/merged/closed per week, issues opened/closed, unique
  contributors/month, bus factor (share of commits by top 3), median time-to-first-response and time-to-merge.
- **Release cadence**: last release per crate, days since, unreleased commits on main since last tag (incl. CHANGELOG gap).
- **CI**: main pass rate per repo, p50 duration, flaky-test leaderboard (jobs that fail then pass on rerun).
- **Traffic**: repo views/clones/referrers (snapshotted), stars/forks trend, GitHub Discussions activity.
- **Code**: size of workspace, dep count, `cargo audit`/`cargo deny` results, MSRV, build time + wasm size of
  reference examples per commit (the original README's perf/size ambition — Phase 3, run in CI of `dioxus`
  and post results to the status API).

### 3.6 Bot workbench (Devin integration)
Uses the Devin REST API (`POST /v1/sessions` with `prompt`, `playbook_id`, `tags`, `title`; `GET /v1/session/{id}`; `POST /v1/session/{id}/message`).
- **Per-PR actions** (buttons in the PR row): each is a playbook + prompt template with PR URL substituted:
  - *Rebase & resolve conflicts*
  - *Address review comments*
  - *Deslop / cleanup* (runs your `deslop-rust-skill` and other org Rust skills)
  - *Add tests for this change*
  - *Deep review* (posts a structured review comment back on the PR — can be the LLM assessment itself)
  - *Split this PR* / *Write changelog entry*
  - *Custom prompt* (free text + pick skills/playbooks from a list fetched from the Devin API)
- **Per-issue actions**: *Reproduce*, *Attempt fix*, *Find duplicates*.
- **Automatic reproductions**: a Devin Automation triggered on `github:issues` (opened, label `bug`, monitored
  repos) starts a session with a "reproduce this issue" playbook: build a minimal repro from the report, run it
  against the current main, and post a comment (reproduced / not reproduced / needs info + steps). The dashboard
  shows repro status per issue and a manual *Reproduce* button for issues the automation skipped. Budget-capped
  per day; opt-in per repo from the settings page.
- **Bulk**: select N PRs → same action → N sessions, tagged `status-dashboard`, tracked in the workbench panel
  (status, resulting PR, ACU cost, link to session). Results also land as a comment on the source PR.
- **Scheduled automations** (Devin Automations or a cron in the collector): nightly "triage new issues",
  weekly "rebase all green contributor PRs", "assess every new PR head".
- Guardrails: confirm dialog + daily ACU budget + org-member auth + audit log table.

### 3.7 Nice-to-haves
- Per-maintainer view (`?me=jkelleyrtp`): your queue, your PRs, your review load.
- Slack/Discord digest: daily summary posted by the server (Today page as text).
- `/api/*.json` endpoints + committed snapshots so the docsite (or a CLI, `dx status`) can consume the data.
- Keyboard-first UI, command palette (`⌘K`) for filters/actions.
- Offline-friendly: whole PR table loads from one cached JSON; search is client-side for snappiness.

## 4. Repo layout (in `DioxusLabs/status`)

```
Cargo.toml                  workspace
packages/
  model/                    shared types (Pr, Issue, Score, Assessment, Metric…) serde + sqlx
  collector/                GitHub/crates.io/docs.rs clients, scoring, LLM assessment, scheduler
  dashboard/                Dioxus 0.8 fullstack app (routes: /, /prs, /issues, /health, /bots, /repo/:name)
  devin/                    thin Devin API client + playbook/prompt templates
rubrics/                    per-repo LLM review rubrics (markdown)
data/                       nightly JSON snapshots (committed by CI)
Dockerfile, fly.toml        same shape as docsite
.github/workflows/          deploy + nightly snapshot commit
```

## 5. Phasing

- **Phase 1 (this session, first PR)**: workspace + Dioxus 0.8 alpha app, collector for PRs (GraphQL) and crates.io
  downloads, SQLite, deterministic score, PR table with filters/search/sorts, Today page, Health page (downloads +
  activity), Dockerfile/fly. Runs locally with a `GITHUB_TOKEN`.
- **Phase 2**: LLM assessment (Claude API) with per-repo rubrics; issues/triage page; Devin workbench with the
  per-PR actions and session tracking; org-member auth.
- **Phase 3**: traffic/CI/release history snapshots, flaky-test tracking, perf/size/build-time ingestion from the
  dioxus CI, Slack/Discord digest, docsite integration, `dx status` CLI.

## 6. Open decisions

1. **LLM provider for assessments**: Claude API directly (cheap, fast, structured output) vs Devin sessions
   (can run tests / use skills, but slower and costs ACUs). Proposal: Claude for assessment, Devin for actions.
2. **GitHub credentials**: GitHub App on the org (recommended) vs a PAT. Traffic stats need push access either way.
3. **Hosting**: fly.io like docsite-playground, or run under dioxuslabs.com later? Proposal: fly now, embed later.
4. **Scope of repos**: all 56, or an explicit allowlist that excludes the Linebender/servo forks (their PRs are
   upstream noise)? Proposal: allowlist in `config.toml`, forks shown in a collapsed "forks" group.
5. **Auth for actions**: GitHub OAuth (org members) vs a shared secret for v1.
