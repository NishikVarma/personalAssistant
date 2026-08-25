# Job Application Copilot

A local-first desktop application that manages the whole outreach loop of a job search —
career profile, contacts, applications, AI-drafted emails, resume tailoring, scheduled
follow-ups and paced bulk campaigns — while **you** approve every consequential action.

Built with **Tauri 2 · React 19 · TypeScript · Rust · SQLite/SQLx**, using **Gemini 2.5
Flash** behind a replaceable `LlmProvider` abstraction. AI generates content only;
deterministic application code owns all state.

Everything runs on your machine: the database is local SQLite, API keys and OAuth refresh
tokens live in your operating system's secure credential storage, and no email is ever
sent without your explicit confirmation.

## Feature tour

### Career profile — the source of truth

- Identity, education, experience, projects, skills, verified resume bullets,
  certifications, achievements and links — each item carries a verified/unverified flag.
- Import an existing resume PDF: text-layer extraction → AI structuring → a review UI
  where you edit/untick items → import through the standard repos **marked verified**.
  Scanned PDFs are detected and fall back to a manual-paste path.
- Every generated email, resume variant and bulk draft draws strictly from this profile;
  inventing experience, technologies or metrics is treated as a bug, not a style choice.

### Contacts & applications

- Lightweight recruiter/referral CRM: organizations, roles, notes, many-to-many tags,
  full-text search and automatic last-contacted stamping when you send an email.
- Application tracker with eleven statuses (`saved → preparing → applied → contacted →
  follow_up_due → response_received → oa → interview → offer / rejected / withdrawn`),
  priority, dates, job URLs, notes and contact linking (with relationship roles).

### AI email drafting & Gmail sending

- Six email types — cold outreach, job application, referral request, follow-up,
  internship inquiry, application status — grounded in a deterministic profile snapshot
  built from every filled-in profile table (~8k char cap).
- **Template memory**: reusable templates are matched by company > role > usage count and
  passed to the prompt as adaptation references; usage counters bump automatically.
- Compose defaults (role + email type) save once and prefill future drafts; the default
  resume is pre-attached and can be swapped per draft; the editor can auto-match the best
  approved variant from the linked application's JD analysis.
- Sending goes through Gmail OAuth after an explicit confirmation dialog, with a
  **duplicate-outreach guard**: addresses emailed within the last 7 days are rejected
  until you explicitly override. Every send is recorded in history.

### Follow-ups that respect reality

- An email sent against an application auto-schedules a follow-up (+N days, configurable,
  optional second round); toggle it per preference in the Follow-ups page.
- Deterministic suppression: the moment a contact replies (detected by Gmail reply sync or
  logged manually) or the application is rejected/withdrawn, pending follow-ups are
  cancelled with a recorded reason — no nagging people who already answered.
- Due / Upcoming / Completed lists, day-precision rescheduling, one-click AI drafts written
  from prior thread context, and a deduplicated desktop notification when the app opens
  with due follow-ups.

### Resumes — honest tailoring

- Immutable, content-addressed storage (sha256 dedup) for master PDFs and Jake's-style
  LaTeX templates; originals are never modified.
- LaTeX engine detection (pdflatex/xelatex/tectonic) gates PDF compilation with a clear
  ".tex export only" fallback.
- **JD matching**: role, seniority, required/preferred skills, matched skills (only ever
  sourced from your profile), honest missing-skill gaps and a recommended resume category.
- **Tailored variants**: the verified profile is projected into your template with
  anti-fabrication rules, written to `variants/{id}.tex` and compiled to PDF when an
  engine exists. Variants are linked to applications, reviewable and approvable.

### Bulk outreach with guardrails

- Import a CSV or XLSX (first sheet): headers + sample-row preview, then column mapping
  with auto-guessed defaults (the email column is required).
- Per-row validation (invalid emails flagged), in-file and existing-contact duplicate
  detection, automatic contact creation, personalized drafts generated per valid recipient
  with template-memory reuse.
- Hit a provider rate limit mid-batch? **Retry failed rows** re-runs only the failures —
  rows that already have a batch draft are skipped — and merges results into the same
  review table.
- Review table shows per-recipient status (ready / invalid / duplicate / failed) with
  draft preview, inline removal, exact send counts and an optional resume attachment
  applied to every email in the run.
- Sending is **frontend-driven and paced**: one explicit confirmation, sequential
  `email_send` calls 2 seconds apart, hard cap of 50 per run (resumable), failures logged
  without aborting; batch totals persist (`bulk_batches`).

### Dashboard & settings

- Dashboard aggregates pipeline counts, due follow-ups and recent activity at a glance.
- Settings: Gemini API key (OS keyring) + model selection + connection test; Gmail
  connect/disconnect and reply sync; compose defaults; follow-up cadence.

## How AI is (and isn't) used

| AI does | Deterministic code owns |
|---|---|
| Drafts emails from your verified profile | Duplicate detection and outreach windows |
| Structures extracted resumes for review | Scheduling, suppression rules, pacing |
| Matches JD skills (profile-sourced only) | Status transitions and rate limiting |
| Projects LaTeX variants from your template | Batch totals, history, notifications |
| Writes follow-ups from thread context | Every send (never without confirmation) |

## Architecture

```
┌───────────────────────────── React 19 + TypeScript ─────────────────────────┐
│  pages/       Dashboard · Contacts · Applications · Emails · Resumes        │
│               CareerProfile · FollowUps · Bulk · Settings                   │
│  lib/ipc.ts   typed wrappers: ipc.{domain}.{action}                         │
│  components/  ui/ (shadcn-style primitives) + domain widgets                │
└───────────────────────────────┬─────────────────────────────────────────────┘
                                │ Tauri IPC (camelCase payloads, string errors)
┌───────────────────────────────┴──────────────── Rust (src-tauri/) ──────────┐
│  commands/   11 domains, thin #[tauri::command] wrappers                    │
│  db/         22 repos owning all SQL + input validation                     │
│  gmail/      oauth.rs (loopback OAuth flow) · mime.rs (RFC 2822, threading) │
│  llm/        LlmProvider trait · GeminiProvider · prompt builders           │
│  error.rs    AppError → serialized to the frontend                          │
│      SQLite via SQLx · migrations 0001–0004                                 │
│      OS keyring via SecretStore: Gemini keys + OAuth refresh tokens         │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **IPC surface**: 122 commands across 11 domains (`settings`, `profile`, `contacts`,
  `applications`, `ai`, `emails`, `gmail`, `follow_ups`, `resumes`, `bulk`), documented in
  [`spec.md`](./spec.md#ipc-surface).
- **Persistence**: SQLite via SQLx with versioned migrations; validation lives in repos,
  never in commands.
- **Secrets**: the `SecretStore` abstraction targets the OS keyring (Windows Credential
  Manager / freedesktop Secret Service); nothing secret is stored in SQLite.
- **Replaceable AI**: swap `LlmProvider` implementations without touching call sites —
  Gemini calls exist only under `src-tauri/src/llm/`.

## Getting started

### Prerequisites

- [Node.js](https://nodejs.org) ≥ 20 and npm
- [Rust](https://rustup.rs) (stable)
- Platform webview dependencies per the [Tauri guide](https://tauri.app/start/prerequisites/):
  - Linux: `webkit2gtk-4.1`, `libappindicator`, etc.
  - Windows: WebView2 (preinstalled on most systems)
  - macOS: Xcode command-line tools
- Optional: a LaTeX engine (`pdflatex`, `xelatex` or `tectonic`) on your `PATH` for
  compiling tailored resume PDFs; otherwise variants export as `.tex`.
- A Gemini API key from [Google AI Studio](https://aistudio.google.com) for AI features —
  stored in the OS keyring. On headless Linux without a Secret Service the key cannot be
  stored.

### Run it

```bash
npm install            # frontend dependencies
cargo fetch            # backend dependencies

npm run tauri dev      # full desktop app in development mode
```

First-run setup inside the app:

1. **Settings → AI provider** — paste your Gemini API key, pick a model, use
   *Test connection*.
2. **Settings → Gmail** — enter your Google OAuth client secret once, hit *Connect*,
   approve the consent page in your browser, and you're ready to send.
3. **Career Profile** — fill in your details or import an existing resume PDF to bootstrap.

## Development

| Command | Location | Purpose |
|---|---|---|
| `npm run tauri dev` | repo root | Run the desktop app |
| `npm test` | repo root | Vitest unit/component tests |
| `npm run build` | repo root | Typecheck + production bundle |
| `cargo test` | `src-tauri/` | Backend integration tests (temp SQLite DBs) |
| `cargo clippy --all-targets` | `src-tauri/` | Lint (must be warning-free) |

A gated live test exercises the real Gemini API:

```bash
GEMINI_API_KEY=... cargo test -- --ignored   # in src-tauri/
```

## Project layout

```
src-tauri/
  src/
    lib.rs          Tauri builder, DB init + migrations, command registration
    models/         serde/sqlx structs (camelCase at the IPC boundary)
    db/             repositories owning all SQL and validation
    commands/       thin command wrappers across 11 domains
    gmail/          OAuth loopback flow, RFC 2822 MIME building, reply sync
    llm/            LlmProvider trait, Gemini client, prompt builders, secrets
    error.rs        AppError (serialized to the frontend)
    tests/          integration tests per domain
  migrations/       SQLx migrations (schema starts in 0001_init.sql)

src/
  lib/ipc.ts        typed IPC wrappers: ipc.{domain}.{action}
  lib/              sections registry, notifications, theme helpers
  pages/            Dashboard, Contacts, Applications, Emails, Resumes,
                    CareerProfile, FollowUps, Bulk, Settings
  components/       ui/ primitives + contacts/, emails/, profile/, resumes/
  __tests__/        Vitest component tests (IPC mocked)
```

Deeper documentation:

- [`spec.md`](./spec.md) — product/technical specification: data model, IPC surface,
  implemented flows, security rules, roadmap matrix
- [`AGENTS.md`](./AGENTS.md) — architecture map and conventions for AI coding agents

## Principles

1. **Local-first** — personal data never leaves the machine except minimal, visible AI
   calls; secrets stay in the OS keyring.
2. **You decide** — emails are never sent automatically; deletes confirm twice; bulk sends
   require one explicit confirmation per batch.
3. **Deterministic code owns state** — duplicates, scheduling, status transitions, rate
   limits and pacing live in application code/database, never in the model.
4. **The verified profile is the truth** — fabrication is treated as a bug.
5. **Originals are immutable** — uploaded resumes/templates are stored read-only;
   generated variants live separately and are reusable.

## Status

All 17 roadmap phases from the original brief are shipped (v0.2.0). Deferred by design:
browser extension / Google Forms assistance, keyboard shortcuts, non-Gmail providers.

## License

Personal project — no license yet.
