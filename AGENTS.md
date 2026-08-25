# AGENTS.md

Instructions for AI coding agents working in this repository.

## Project

Job Application Copilot — a local-first desktop app (Tauri 2 + React 19 + TypeScript +
Rust + SQLite/SQLx) that assists with job applications and outreach. AI (Gemini 2.5 Flash)
assists; deterministic code owns all state. Full product/technical spec: `spec.md`.

## Commands

```bash
# frontend / root
npm install            # first time
npm run dev            # vite dev server (tauri dev runs this automatically)
npm test               # vitest run
npm run build          # tsc && vite build

# backend (run inside src-tauri/)
cargo test
cargo clippy --all-targets

# full app during development
npm run tauri dev
```

## Architecture Map

Status: all 17 roadmap phases shipped (v0.2.0); see the roadmap table in `spec.md`.

Backend (`src-tauri/src/`):

- `lib.rs` — Tauri builder, DB init + migrations (0001–0004), command registration.
- `models/` — serde/sqlx structs. One module per domain. camelCase at the IPC boundary.
- `db/` — repos own all SQL and input validation; one file per domain (contacts, applications,
  profile, resumes, resume_variants, generated_emails, bulk_batches, email_history, follow_ups,
  settings, ...); shared helpers (`now`, `required`, `optional`) in `db/mod.rs`.
- `commands/` — thin `#[tauri::command]` wrappers across 11 domains (ai, applications, bulk,
  contacts, emails, follow_up, gmail, profile, resumes, settings); register every command in `lib.rs`.
- `gmail/` — Gmail integration: `oauth.rs` (device-code OAuth flow, token persistence),
  `mime.rs` (RFC 2822 MIME building with attachments, reply threading, sent-message sync).
- `llm/` — the only place Gemini is called: `LlmProvider` trait in `mod.rs`, prompt builders
  (`email_prompt.rs`, `extract_prompt.rs`, `jd_prompt.rs`), API-key handling in `secrets.rs`.
- `error.rs` — `AppError`; serializes to a string for the frontend.
- `tests/` — integration tests per domain against temp SQLite DBs.

Frontend (`src/`):

- `lib/ipc.ts` — typed interfaces + invoke wrappers grouped as `ipc.{domain}.{action}`.
- `pages/` — Dashboard, Contacts, Applications, Emails, Resumes, CareerProfile, FollowUps,
  Bulk, Settings.
- `components/` — `ui/` holds shadcn-style primitives; `contacts/`, `emails/`, `profile/`,
  `resumes/` hold domain widgets.
- `lib/sections.ts` + `App.tsx` — sidebar registry and route wiring; `lib/notifications.ts`
  (toast system), `lib/theme.ts` (light/dark).

## Key Flows

- **Email send**: `commands/emails.rs::email_send` → repo status checks (approved only) →
  `gmail/mime.rs` builds RFC 2822 → Gmail REST send → transactional history row +
  draft `sent` flip + contact last-contacted stamp.
- **Bulk send**: frontend-paced loop over batch recipients calling `email_send`
  (2s apart, cap 50/run); failures log without aborting; totals persist via
  `bulk_batch_finish`; `bulk_retry_failed` re-generates only failed rows that lack drafts.
- **Reply sync**: `gmail_sync_replies` polls threads of awaiting sends; incoming messages
  become `email_history` rows, sent rows flip to `replied`, pending follow-ups suppress.

## Adding a New Entity (checklist)

1. Model + input struct in `src-tauri/src/models/<domain>.rs` (derive Serialize, Deserialize,
   sqlx::FromRow; `#[serde(rename_all = "camelCase")]`). Use the `str_enum!` macro for enums.
2. Repo in `src-tauri/src/db/<entity>_repo.rs` following existing repos: validate inputs there
   (`required`/`optional`), map unique violations to `AppError::InvalidInput`, return
   `NotFound` with entity + id, preserve `created_at` on update, clean up polymorphic child
   rows transactionally when the parent table lacks FKs.
3. Thin commands in `src-tauri/src/commands/<domain>.rs`; register them in `lib.rs`.
4. Rust tests in `src-tauri/tests/<domain>_test.rs` (CRUD roundtrip, validation failures,
   duplicate handling, cascade/cleanup behavior).
5. Types + wrappers in `src/lib/ipc.ts`.
6. UI: use `SectionCard`, `FormDialog` (schema-driven fields), `VerifiedToggle`,
   `DeleteButton`. Wire the route in `App.tsx` and remove the `phase` marker in
   `src/lib/sections.ts` when the section becomes real.
7. Vitest component test mocking `@tauri-apps/api/core`.

## Conventions & Hard Rules

- No code comments unless asked.
- Validation lives in repos, not commands. Commands contain no business logic.
- All IPC payloads are camelCase; Rust enum values serialize as snake_case strings.
- Frontend deletes are two-step confirms (`DeleteButton`); never use native `confirm()`.
- NEVER send an email or take any external action without explicit user confirmation;
  bulk sends always require confirmation and are never automatic.
- The AI never owns deterministic state: duplicate detection, scheduling, status
  transitions, rate limiting stay in application code/database.
- The career profile is the source of truth; never generate content claiming experience,
  technologies, metrics, or achievements not present in verified data.
- Original uploaded resumes/templates are immutable; generated variants go to
  `resume_variants`.
- Keep the LLM provider replaceable via `LlmProvider`; do not hardcode Gemini calls outside
  `src-tauri/src/llm/`.
- Do not implement future-phase features prematurely (see roadmap in `spec.md`).
- Keep existing working code stable; prefer small verifiable changes.

## Before Committing

Run and pass:

```bash
npx tsc --noEmit
npm test
npm run build
cargo test          # in src-tauri/
cargo clippy --all-targets   # no warnings
```

Commit messages follow conventional style used in history: `feat:`, `fix:`, `chore:`,
`test:`, `docs:`. Never commit secrets or API keys.
