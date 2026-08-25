# Job Application Copilot

A local-first desktop application that assists with job applications and outreach.
Built with **Tauri 2 · React 19 · TypeScript · Rust · SQLite/SQLx**, with AI assistance from
**Gemini 2.5 Flash** behind a replaceable `LlmProvider` abstraction.

Everything runs on your machine: the database is local SQLite, API keys live in your
operating system's secure credential storage, and nothing is ever sent without your explicit
confirmation.

## Current capabilities — the full loop

- **Career profile** — identity, education, experience, projects, skills, resume bullets,
  certifications, achievements and links, each with verified/unverified flags. Import an
  existing resume PDF and the AI structures it for your review. This is the single source
  of truth every generated email and resume draws from.
- **Contacts** — recruiters/referrals with organizations, roles, notes, tags, search and
  last-contacted tracking.
- **Applications** — track opportunities through eleven statuses (`saved` → `applied` →
  `interview` → `offer` …) with priority, dates, job URLs and notes.
- **AI email drafts** — generate cold outreach, applications, referral requests,
  follow-ups, internship inquiries or status checks grounded strictly in your verified
  profile, then send directly via Gmail (with attachments) after an explicit confirmation.
- **Follow-ups** — scheduled automatically after outreach, suppressed the moment a contact
  replies, with desktop notifications and one-click AI-drafted follow-ups.
- **Resumes** — immutable master PDF + LaTeX template storage; AI tailors your template to
  a job description (compiles to PDF when LaTeX is installed); JD↔profile matching shows
  honest skill gaps.
- **Bulk outreach** — import a CSV/XLSX, map columns, generate personalized drafts for
  every recipient, review, and send in paced batches (2s apart, max 50 per run) behind an
  explicit confirmation.

Intentionally out of scope for v1: browser extension / Google Forms assistance, keyboard
shortcuts, non-Gmail providers.

## Prerequisites

- [Node.js](https://nodejs.org) ≥ 20 and npm
- [Rust](https://rustup.rs) (stable)
- Platform webview dependencies per the [Tauri guide](https://tauri.app/start/prerequisites/):
  - Linux: `webkit2gtk-4.1`, `libappindicator`, etc.
  - Windows: WebView2 (preinstalled on most systems)
- A Gemini API key ([Google AI Studio](https://aistudio.google.com)) if you want email
  generation. Stored in the OS keyring (Windows Credential Manager / freedesktop Secret
  Service); on headless Linux without a Secret Service the key cannot be stored.

## Getting started

```bash
npm install            # frontend dependencies
cargo fetch            # backend dependencies

npm run tauri dev      # run the full desktop app in development mode
```

Then open **Settings → AI provider** and paste your Gemini API key. Use *Test connection*
to verify it works.

## Commands

| Command              | Location     | Purpose                          |
|----------------------|--------------|----------------------------------|
| `npm run tauri dev`  | repo root    | Run the desktop app              |
| `npm test`           | repo root    | Vitest unit/component tests      |
| `npm run build`      | repo root    | Typecheck + production bundle    |
| `cargo test`         | `src-tauri/` | Backend integration tests        |
| `cargo clippy --all-targets` | `src-tauri/` | Lint (must be warning-free) |

A gated live test exercises the real Gemini API:

```bash
GEMINI_API_KEY=... cargo test -- --ignored   # in src-tauri/
```

## Project layout

```
src-tauri/src/
  lib.rs          Tauri builder, DB init, command registration
  models/         serde/sqlx structs (camelCase at the IPC boundary)
  db/             repositories owning all SQL and validation
  commands/       thin #[tauri::command] wrappers
  llm/            LlmProvider trait, Gemini client, prompt builders
  migrations/     SQLx migrations (full schema lives in 0001_init.sql)

src/
  lib/ipc.ts      typed IPC wrappers (ipc.{domain}.{action})
  pages/          Dashboard, CareerProfile, Contacts, Applications, Emails, Settings
  components/ui/  shadcn-style primitives
```

Deeper documentation:

- [`spec.md`](./spec.md) — product/technical specification, data model, roadmap matrix
- [`AGENTS.md`](./AGENTS.md) — conventions and checklists for AI coding agents

## Principles

1. Local-first: personal data never leaves the machine except minimal, visible AI calls.
2. The user controls all consequential actions; emails are never sent automatically.
3. Deterministic code owns state (duplicates, scheduling, status transitions, rate limits);
   AI only generates content.
4. The verified career profile is the source of truth; fabrication is treated as a bug.
5. Original uploaded resumes/templates are immutable; generated variants are stored
   separately and reusable.

## License

Personal project — no license yet.
