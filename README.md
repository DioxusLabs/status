# status of dioxus org

This uses doxie-bot to track that status of things in the dioxus org.

This includes:
- Performance benchmarks
- Size, build times, and memory usage
- Code quality
- Security
- Dependencies
- Changed crates
- Open issues
- Open PRs
- etc

Whenever a change is merged on PRs that this repo watches, we re-run the bot to generate a new blob of status data which you can then view on the dioxuslabs.com website.

For now, we just use a github pages site to host and preview the data, but we do eventually want to move this to dioxuslabs.com and just query the data in this repo.

## Development

The app crate is `packages/status` (`dioxus-status`). `dx` builds with the cargo
profiles `server-dev` (host) and `wasm-dev` (wasm32); they are declared in the
workspace `Cargo.toml` so plain cargo accepts them and dx reuses the same
artifacts instead of compiling the tree twice.

```sh
# server (host) — the --target flag matters: dx always passes it, so its
# artifacts live in target/x86_64-unknown-linux-gnu/server-dev
cargo check  -p dioxus-status --features server --profile server-dev --target x86_64-unknown-linux-gnu
cargo clippy -p dioxus-status --features server --profile server-dev --target x86_64-unknown-linux-gnu -- -D warnings
cargo test   -p dioxus-status --features server --profile server-dev --target x86_64-unknown-linux-gnu

# web (wasm)
cargo check  -p dioxus-status --features web --target wasm32-unknown-unknown --profile wasm-dev
cargo clippy -p dioxus-status --features web --target wasm32-unknown-unknown --profile wasm-dev -- -D warnings
```

Note: `cargo test` still compiles a parallel dep tree once — the `dioxus-ssr`
dev-dependency enables extra `dioxus` features, changing fingerprints relative
to dx's build. `check`/`clippy`/`build` share dx's dep artifacts.

Run the dev server:

```sh
dx serve --package dioxus-status --web --port 8080 --hot-patch false --hot-reload false
```

Copy `.env.example` to `.env` and fill in `GITHUB_TOKEN`/`ADMIN_TOKEN` (and the
GitHub App / Devin vars for those features).
