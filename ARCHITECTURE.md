# Architecture (implementation notes)

Single Dioxus 0.8 fullstack crate, modelled on `skyvm-web` (see /home/ubuntu/repos/skyvm/packages/skyvm-web).

```
Cargo.toml                      workspace: packages/status, vendor/*
packages/status/
  Cargo.toml                    name = "dioxus-status"; features web / server
  Dioxus.toml
  assets/                       main.css, dx-components-theme.css (from dioxus-components 0.8 branch)
  src/
    main.rs                     Route enum, App, launch; server: init db + start collectors
    components/                 vendored styled components from dioxus-components (button, badge, card,
                                input, select, tabs, dialog, tooltip, checkbox, switch, progress, separator,
                                dropdown_menu, toast, table-ish)  — copied from preview/src/components
    model/                      shared serde types (Pr, Issue, Repo, Score, Assessment, CrateStats, ...)
    api/                        server fns: #[get]/#[post]; the only client<->server boundary
    backend/  (cfg server)      db.rs (sqlx sqlite + migrations/), github.rs (octocrab GraphQL/REST),
                                crates_io.rs, scoring.rs, collector.rs (scheduler), devin.rs, anthropic.rs,
                                auth.rs (admin session cookie / github oauth)
    ui/                         pages + shared layout
vendor/
  dioxus-primitives/            copied from DioxusLabs/dioxus-components branch 0.8 (primitives/)
  dioxus-attributes/            same repo (dioxus-attributes/)
migrations/                     sqlx sql migrations
Dockerfile, fly.toml
```

Env vars: `GITHUB_TOKEN` (fallback), `GITHUB_APP_ID`, `GITHUB_APP_PRIVATE_KEY` or `GITHUB_APP_PRIVATE_KEY_PATH`,
`GITHUB_APP_INSTALLATION_ID` (optional; discovered via `/app/installations`), `DEVIN_API_KEY`, `DEVIN_ORG_ID`,
`DEVIN_API_BASE` (default https://api.devin.ai; calls go to `{base}/v3/organizations/{org}/sessions`), `ANTHROPIC_API_KEY`,
`LLM_DAILY_BUDGET` (default 50; overridable via `settings.llm_daily_budget`),
`ADMIN_TOKEN` (shared secret for mutation auth in v1), `DATA_DIR` (default `./_data`), `PORT`.

Auth model: all pages public. Mutations (settings changes, dispatch Devin, assess with LLM, roadmap admin) require an
admin session (cookie set by `/settings` login with `ADMIN_TOKEN`; GitHub OAuth org-member login is the follow-up).
Public voting on roadmap items needs no auth (anonymous voter id cookie + rate limit).

LLM policy: never called in background. Only the explicit "Assess" action calls Anthropic, result cached per
(pr, head_sha); daily cap `LLM_DAILY_BUDGET` (default 50 calls).
