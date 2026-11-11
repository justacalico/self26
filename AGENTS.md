# self26

A self-hosted e621-compatible media server. Rust backend (axum + rusqlite)
serves the e621 API (legacy and v2 formats) and embeds the Flutter web UI.
The Flutter app also builds for Android, iOS, Linux, Windows and macOS as a
client that talks to any self26 server.

- Keep modules focused; refactor shared logic instead of duplicating it.
- Aim for a grade-A codebase with 100% Flutter line coverage.
- All user-facing strings go through the ARB l10n files (en, en_GB, zh, zh_Hans, ru, ga).
- The API must stay compatible with the current e621 v2 API shapes.
- Backend verification: `cargo test`. Frontend: `flutter test --coverage`.
- Conventional commits only; `cog check` must pass (git hooks: `git config core.hooksPath .githooks`).
- Never commit keystores, key.properties, GPG private keys, or other secrets.

## Layout

- `src/` Rust server: api/, db, search, auth, media, embedded assets.
- `lib/` Flutter app: pages, shell, state, api client, l10n (generated files are gitignored).
- `landing/` Flutter web landing page deployed to GitLab Pages at `/`; the app is at `/app/`.
- `scripts/` build, release, sync and packaging helpers.
- `packaging/`, `flatpak/`, `repos/` platform packaging metadata.
- `frontend/dist/` embedded web build output (placeholder is committed; real build via scripts/build-flutter.sh).
