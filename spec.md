# Job Application Copilot — Product & Technical Spec

A cross-platform (Linux/Windows) local-first desktop app that automates repetitive job
application and outreach tasks while requiring explicit user confirmation before any
consequential action (e.g. sending email).

Stack: **Tauri 2 · React 19 + TypeScript · Rust · SQLite/SQLx · Gemini 2.5 Flash behind a
replaceable `LlmProvider` abstraction · Gmail API/OAuth 2.0**.

## Architectural Principles

- Local-first personal application; personal data stays on the machine.
- User remains in control of external actions: never send an email, submit an application,
  or perform outreach without explicit confirmation.
- AI assists; deterministic code controls state (dupes, scheduling, status transitions,
  validation, rate limiting).
- Career profile is the source of truth; the AI must never claim information not present
  in the verified profile.
- Original uploaded resumes are immutable; generated variants are stored separately and
  are reusable.
- Avoid unnecessary AI/API calls and regeneration; reuse templates and prior content.
- Keep the LLM provider replaceable; design for future providers (Ollama/local) without
  implementing them prematurely.
- Prefer simple architecture over microservices/agent frameworks.

## Architecture

```
src-tauri/                  Rust backend (Tauri v2)
  src/
    lib.rs                  Tauri builder; DB init at startup; command registration
    main.rs                 entry point
    state.rs                AppState { SqlitePool, db_path }
    error.rs                AppError (thiserror) -> serialized as string to frontend
    llm/mod.rs              LlmProvider trait + GeminiProvider stub (Phase 6)
    models/                 serde + sqlx::FromRow structs; camelCase at IPC boundary
      profile.rs            profile entities + str_enum! macro (EmploymentType, …)
      contact.rs            Contact / Tag
      application.rs        ApplicationStatus enum + Application
    db/
      mod.rs                pool setup, migrations (sqlx::migrate!), shared validators
      *_repo.rs             one repo per domain; validation lives here
    commands/               thin #[tauri::command] wrappers over repos
  migrations/0001_init.sql  full normalized schema (25 tables)
  tests/                    db_test.rs, profile_test.rs, contacts_test.rs, applications_test.rs

src/                        React frontend
  lib/ipc.ts                typed interfaces + invoke wrappers grouped by domain
  lib/sections.ts           sidebar section registry (implemented vs placeholder phases)
  pages/                    Dashboard, Settings, CareerProfile, Contacts, Applications
  components/ui/            shadcn-style primitives (button, card, input, dialog, …)
  components/profile/       shared CRUD building blocks (FormDialog, SectionCard,
                            VerifiedToggle, DeleteButton, BulletsEditor, …)
  components/contacts/      contact-specific widgets
```

Patterns worth knowing:

- All IPC payloads are `camelCase` (serde `rename_all`); Rust internals use snake_case.
- Typed string enums (via `str_enum!`) serialize as snake_case strings both ways.
- Repos own validation (`required`, `optional`, email format, unique-violation mapping to
  friendly `InvalidInput` errors). Commands stay thin.
- `bullets` / `entity_skills` reference projects/experiences polymorphically without FKs;
  parent deletes clean them up transactionally inside the repo.
- Frontend deletes use a two-step confirm button (native `confirm()` is unreliable in webviews).

## Data Model (SQLite, migration `0001_init.sql` — 25 tables)

Career profile: `user_profile` (single row id=1), `education`, `experience`, `projects`,
`skills`, `entity_skills` (project/experience ↔ skill), `bullets` (verified resume bullets
per project/experience), `certifications`, `achievements`, `links`.

Resumes: `resume_files` (immutable pdf_master/tex_template), `resume_variants`,
`variant_sections`, `variant_bullets`.

Jobs: `applications` with status enum `saved | preparing | applied | contacted |
follow_up_due | response_received | oa | interview | offer | rejected | withdrawn`.

Contacts: `contacts` (UNIQUE email), `tags`, `contact_tags`, `application_contacts`
(relationship + is_primary).

Emails: `email_templates` (email_type enum, variables, success tracking), `generated_emails`
(draft → edited → approved → sent/discarded), `email_history` (direction, gmail ids,
delivery method, response status).

Workflow: `follow_ups` (sequence, scheduled_for, suppression reasons).

Platform: `settings` (key/value), `oauth_accounts` (provider metadata only — tokens live in
the OS keyring; Gmail passwords are never stored).

## IPC Surface

121 commands registered in `src-tauri/src/lib.rs`:

- settings (4): get/set/delete_setting, get_app_info
- profile (43): profile_get/update/set_verified; education/experience/project CRUD +
  set_verified + delete; skill CRUD + list_for_entity + replace_for_entity;
  bullet create/update/set_verified/delete/list_for_entity; certification /
  achievement / link CRUD
- contacts (10): contact_list(search)/create/update/delete/set_last_contacted;
  tag_list/create/delete; contact_list_tags/replace_tags
- applications (5): application_list(status filter)/create/update/set_status/delete
- ai (5): ai_get_config / set_model / set_api_key / clear_api_key / test_connection
- emails (17): generated_email_list/get/create/update/set_status/delete/send/
  save_as_template; email_history_list/set_response/record_incoming;
  email_template_list/create/update/delete; ai_generate_email/ai_extract_contact
- gmail (7): google_set_client_secret / has_client_secret / begin_connect /
  complete_connect / status / disconnect / sync_replies
- follow_ups (9): follow_up_list / due / due_count / sweep / reschedule / cancel /
  config_get / config_set / draft
- resumes (14): resume_file_upload (content-addressed, dedup) / list / delete /
  tex_content / latex_detect / extract_profile / extract_from_text /
  profile_import_extracted / resume_match_jd / resume_generate_variant /
  resume_variant_list / resume_variant_tex_content / resume_variant_approve /
  resume_variant_delete
- bulk (7): bulk_import_preview / batch_create / batch_list / batch_get /
  generate / batch_remove_draft / batch_finish

Frontend wrappers mirror these in `src/lib/ipc.ts` under `ipc.{domain}.{action}`.

## AI Layer

- `LlmProvider` trait (async `complete(prompt) -> String`) in `src-tauri/src/llm/mod.rs`;
  `GeminiProvider` calls the Gemini REST `generateContent` endpoint via reqwest (rustls).
- API keys live in the OS keyring behind the `SecretStore` abstraction
  (`llm/secrets.rs`: `KeyringStore` for Windows Credential Manager / freedesktop Secret
  Service, `MemoryStore` for tests). Keys are never stored in SQLite.
- The model name is a plain setting (`ai.model`, default `gemini-2.5-flash`).
- A live network round-trip test exists but is `#[ignore]`-gated: run manually with
  `GEMINI_API_KEY=... cargo test -- --ignored`.

## Email Generation Flow (implemented)

`ai_generate_email` assembles a prompt from (a) the request details (recipient, company,
role, job description, context, email type) and (b) a deterministic profile snapshot built
from every filled-in career-profile table (`db/profile_snapshot.rs`, capped ~8k chars). The
prompt forbids inventing anything absent from that snapshot. The model returns strict JSON
`{subject, body}`, parsed tolerantly (`llm/email_prompt.rs`). Drafts persist in
`generated_emails` with recipient + provider/model provenance; `draft → edited → approved →
sent` transitions are validated in the repo (`sent`/`discarded` are terminal). Existing
contacts are auto-linked by exact email match.

## Gmail Sending (implemented)

- OAuth 2.0 desktop loopback flow: `google_begin_connect` binds a local listener and returns
  the consent URL (scopes: `openid email gmail.send`); the frontend opens the browser and
  `google_complete_connect` exchanges the code. The **refresh token lives in the OS keyring**
  via `SecretStore`; `oauth_accounts` stores only account metadata. Client secret is also
  keyring-backed.
- `email_send` requires `approved` status, resolves the recipient (stored on the draft or
  via the linked contact), refreshes the access token, builds RFC 2822 MIME (multipart when
  a file is attached), sends through the Gmail REST API, then transactionally records the
  send in `email_history`, flips the draft to `sent` and stamps the contact's
  last-contacted time.
- **Duplicate-outreach guard**: sends to an address already emailed within 7 days are
  rejected (`AppError::RecentOutreach`) until the user explicitly overrides in the
  confirmation dialog.
- Migration `0002_email_recipients.sql` adds recipient columns to `generated_emails` and
  `email_history`.

## Resume Import & Extraction (implemented)

- Master resume PDFs and Jake's-style .tex templates upload into an immutable,
  content-addressed store (`app_data_dir/resumes/`, sha256 dedup). Originals are never
  modified; deleting removes only the stored copy.
- LaTeX engine detection (pdflatex/xelatex/tectonic) gates later PDF compilation with a
  clear ".tex export only" fallback.
- `resume_extract_profile` pulls the PDF text layer (scanned PDFs are rejected with a
  paste-fallback path), sends it to Gemini with strict facts-only rules, and returns a
  structured `ExtractedProfile`. The review UI lets you edit, untick and import; approved
  items are created through the standard profile repos **marked verified**.

## JD Matching & Tailored Generation (implemented)

- `resume_match_jd` analyzes a job description against the verified profile: role,
  seniority, required/preferred skills, **matched skills (only from the profile)** and
  honest missing-skill gaps, plus a recommended resume category.
- `resume_generate_variant` projects the verified profile into the user's .tex template
  (structure preserved, anti-fabrication rules enforced), writes `variants/{id}.tex` and
  compiles to PDF via the detected engine — `.tex`-only fallback otherwise. Variants are
  linked to applications, reviewable and approvable on the Resumes page.
- Compose integration: drafts pre-attach the user's default resume (set per PDF on the
  Resumes page); the editor can auto-match the best variant from the linked application's
  JD. Compose defaults (role + email type) are saved once and reused.

## Bulk Outreach (implemented)

- Import a CSV or XLSX (`csv` + `calamine`, first sheet): headers + sample rows preview,
  then column mapping with auto-guessed defaults (email column required).
- `bulk_generate` validates every row (invalid emails flagged), detects in-file and
  existing-contact duplicates, auto-creates contact rows, and generates a personalized
  draft per valid recipient — grounded in the verified profile with template memory reuse.
- The review table shows per-recipient status (ready / invalid / duplicate / failed) with
  draft preview, inline removal and exact send counts.
- Sending is **frontend-driven and paced**: explicit confirmation, sequential
  `email_send` calls 2 seconds apart, hard cap of 50 per run (resumable), failures logged
  without aborting. Batch totals and status are persisted (`bulk_batches`).

## Follow-up System (implemented)

- Every sent email linked to an application auto-schedules a follow-up (+N days,
  configurable, default 7; optional second round). Toggle in the Follow-ups page.
- Deterministic suppression: contact replied (detected by reply sync or manual logging) or
  application rejected/withdrawn -> pending follow-ups are suppressed with a reason.
- The Follow-ups page shows Due / Upcoming / Completed lists with draft-generation
  (AI writes from prior thread context), day-precision rescheduling and cancellation.
- Desktop notification fires when the app opens with due follow-ups (deduped per day).

## Email History, Templates & Reply Sync (implemented)

- Every send is recorded in `email_history` (direction, Gmail ids, recipient, response
  status `awaiting`); the linked contact's last-contacted time is stamped transactionally.
- The Emails page has a **History** card: filter by application or contact, per-row
  response status dropdown (awaiting / replied / no reply needed), manual **Log received**
  dialog, and **Sync replies**.
- **Reply sync** (`gmail_sync_replies`) polls the Gmail threads of up to 50 `awaiting`
  sends: a message from a different sender newer than our send is recorded as an incoming
  history row (snippet as body preview) and the sent row flips to `replied`. Detection is
  deterministic application code; the AI is never involved.
- **Template memory**: `email_templates` stores reusable emails (user-created or saved
  from drafts). `ai_generate_email` picks the best match for the draft's type — company
  match > role match > usage count — passes it into the prompt as an adaptation reference
  and bumps its usage counters. The UI offers a Templates library card and a
  "Save as template" action on approved/sent drafts.

## Implemented Capabilities (as of this spec)

Development plan from the original project brief — 17 incremental steps:

| #  | Step                                            | Status |
|----|-------------------------------------------------|--------|
| 1  | Project setup (Tauri/React/Rust integration)    | Done   |
| 2  | SQLite/database layer                           | Done   |
| 3  | Career profile                                  | Done   |
| 4  | Contact management                              | Done   |
| 5  | Basic application tracking                      | Done   |
| 6  | Gemini integration (LLMProvider implementation) | Done   |
| 7  | Single-email generation                         | Done   |
| 8  | Gmail OAuth and sending                         | Done   |
| 9  | Email history                                   | Done   |
| 10 | Follow-up scheduling/notifications              | Done   |
| 11 | Resume PDF import                               | Done   |
| 12 | Career profile extraction/verification          | Done   |
| 13 | LaTeX template import                           | Done   |
| 14 | Resume generation                               | Done   |
| 15 | Resume/JD matching                              | Done   |
| 16 | Bulk CSV/XLSX outreach                          | Done   |
| 17 | Testing, error handling, security hardening     | Done   |

Deferred by design (not defects): Gmail reply threading beyond the poll-based sync;
browser extension / Google Forms assistance; keyboard shortcuts. All core flows
(history, dashboard aggregation, outreach warnings) are implemented.

Intentionally out of scope for v1: browser extension / Google Forms support,
keyboard shortcuts, non-Gmail email providers.

## Security & Privacy Rules

- Send only necessary information to Gemini; clearly indicate when data leaves the machine.
- OAuth tokens go to OS secure credential storage; never store Gmail passwords.
- Validate email addresses before sending; warn on duplicate/recent outreach.
- Never fabricate technologies, experience, metrics, or achievements.

## Testing

- Rust: `cargo test` in `src-tauri` (repo-level tests against temp SQLite databases).
- Frontend: Vitest + Testing Library (`npm test`), IPC mocked via `vi.mock("@tauri-apps/api/core")`.
- Verify before committing: `npx tsc --noEmit`, `npm test`, `npm run build`, `cargo test`,
  `cargo clippy --all-targets`.
