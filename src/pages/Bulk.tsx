import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Layers, RefreshCw, Send, Sparkles, Upload } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import DeleteButton from "@/components/profile/DeleteButton";
import EmptyState from "@/components/EmptyState";
import SectionCard from "@/components/profile/SectionCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import {
  EMAIL_TYPES,
  ipc,
  type Application,
  type BulkBatch,
  type BulkColumnMapping,
  type BulkImportPreview,
  type BulkRowStatus,
  type EmailType,
  type ResumeFile,
} from "@/lib/ipc";

const EMAIL_TYPE_LABELS: Record<EmailType, string> = {
  cold_outreach: "Cold outreach",
  job_application: "Job application",
  referral_request: "Referral request",
  follow_up: "Follow-up",
  internship_inquiry: "Internship inquiry",
  application_status: "Application status",
};

const MAPPABLE_FIELDS: { key: keyof BulkColumnMapping; label: string; required?: boolean }[] = [
  { key: "email", label: "Email", required: true },
  { key: "name", label: "Name" },
  { key: "company", label: "Company" },
  { key: "role", label: "Role" },
  { key: "jobDescription", label: "Job description" },
];

const SEND_DELAY_MS = 2000;
const SEND_CAP = 50;

function guessColumn(headers: string[], field: string): string {
  const lower = headers.map((h) => h.toLowerCase());
  const patterns: Record<string, string[]> = {
    email: ["email", "e-mail", "mail"],
    name: ["name", "contact", "full name"],
    company: ["company", "organization", "organisation", "firm"],
    role: ["role", "title", "position", "job title"],
    jobDescription: ["job description", "description", "jd", "posting"],
  };
  for (const pattern of patterns[field] ?? []) {
    const hit = lower.find((h) => h.includes(pattern));
    if (hit !== undefined) return headers[lower.indexOf(hit)];
  }
  return "";
}

type Step = "upload" | "map" | "review";

export default function Bulk() {
  const navigate = useNavigate();
  const [step, setStep] = useState<Step>("upload");
  const [sourcePath, setSourcePath] = useState("");
  const [preview, setPreview] = useState<BulkImportPreview | null>(null);
  const [mapping, setMapping] = useState<BulkColumnMapping>({
    name: "",
    email: "",
    company: "",
    role: "",
    jobDescription: "",
  });
  const [emailType, setEmailType] = useState<EmailType>("job_application");
  const [applicationId, setApplicationId] = useState("");
  const [applications, setApplications] = useState<Application[]>([]);
  const [batch, setBatch] = useState<BulkBatch | null>(null);
  const [rows, setRows] = useState<BulkRowStatus[]>([]);
  const [generating, setGenerating] = useState(false);
  const [sending, setSending] = useState(false);
  const [sendProgress, setSendProgress] = useState<{ done: number; total: number } | null>(null);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [previewDraft, setPreviewDraft] = useState<{ subject: string; body: string } | null>(null);
  const [resumeFiles, setResumeFiles] = useState<ResumeFile[]>([]);
  const [defaultResumeId, setDefaultResumeId] = useState<number | null>(null);
  const [attachment, setAttachment] = useState<{ path: string; name: string } | null>(null);
  const [retrying, setRetrying] = useState(false);

  useEffect(() => {
    loadApplications();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const loadApplications = () => {
    ipc.application
      .list()
      .then((rows) => setApplications(Array.isArray(rows) ? rows : []))
      .catch(() => setApplications([]));
    ipc.resumeFile
      .list("pdf_master")
      .then((rows) => {
        const files = Array.isArray(rows) ? rows : [];
        setResumeFiles(files);
        // pre-select the default resume (fallback: most recent)
        ipc
          .getSetting("compose.default_resume_id")
          .then((v) => {
            const defaultId = v ? Number(v) : null;
            setDefaultResumeId(defaultId);
            const chosen =
              files.find((f) => f.id === defaultId) ?? files[0];
            if (chosen) {
              setAttachment({ path: chosen.storedPath, name: chosen.originalFilename });
            }
          })
          .catch(() => {
            const chosen = files[0];
            if (chosen) setAttachment({ path: chosen.storedPath, name: chosen.originalFilename });
          });
      })
      .catch(() => setResumeFiles([]));
  };

  const pickFile = async () => {
    const selection = await open({
      multiple: false,
      title: "Import contacts",
      filters: [{ name: "Spreadsheet", extensions: ["csv", "xlsx", "xls"] }],
    });
    if (typeof selection !== "string") return;
    try {
      const imported = await ipc.bulk.importPreview(selection);
      setSourcePath(selection);
      setPreview(imported);
      // auto-guess mappings
      setMapping({
        name: guessColumn(imported.headers, "name"),
        email: guessColumn(imported.headers, "email"),
        company: guessColumn(imported.headers, "company"),
        role: guessColumn(imported.headers, "role"),
        jobDescription: guessColumn(imported.headers, "jobDescription"),
      });
      loadApplications();
      setStep("map");
    } catch (e) {
      toast.error(String(e));
    }
  };

  const generate = async () => {
    if (!mapping.email) {
      toast.error("Map the email column first.");
      return;
    }
    setGenerating(true);
    try {
      const created = await ipc.bulk.batchCreate(emailType, applicationId ? Number(applicationId) : null);
      setBatch(created);
      const statuses = await ipc.bulk.generate(
        created.id,
        sourcePath,
        mapping,
        { role: null, company: null, jobDescription: null },
      );
      setRows(Array.isArray(statuses) ? statuses : []);
      setStep("review");
      const ready = statuses.filter((s) => s.status === "ready").length;
      toast.success(`Generated ${ready} personalized draft${ready === 1 ? "" : "s"}`);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setGenerating(false);
    }
  };

  const retryFailed = async () => {
    if (!batch) return;
    setRetrying(true);
    try {
      const retried = await ipc.bulk.retryFailed(batch.id, sourcePath, mapping, {
        role: null,
        company: null,
        jobDescription: null,
      });
      const list = Array.isArray(retried) ? retried : [];
      // merge: replace rows we just retried, keep ready rows untouched
      setRows((prev) => {
        const byEmail = new Map(list.map((r) => [r.email.toLowerCase(), r]));
        return prev.map((row) => {
          if (row.status !== "failed" && row.status !== "invalid") return row;
          const updated = byEmail.get(row.email.toLowerCase());
          return updated ?? row;
        });
      });
      const nowReady = list.filter((r) => r.status === "ready").length;
      toast.success(`Retried ${list.length} recipient${list.length === 1 ? "" : "s"} — ${nowReady} now ready`);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setRetrying(false);
    }
  };

  const removeRecipient = async (row: BulkRowStatus) => {
    if (!batch || !row.generatedEmailId) return;
    try {
      await ipc.bulk.removeDraft(batch.id, row.generatedEmailId);
      setRows((prev) => prev.filter((r) => r.generatedEmailId !== row.generatedEmailId));
      toast.success("Recipient removed from the batch");
    } catch (e) {
      toast.error(String(e));
    }
  };

  const previewRecipient = async (draftId: number) => {
    try {
      const draft = await ipc.generatedEmail.get(draftId);
      setPreviewDraft({ subject: draft.subject ?? "(no subject)", body: draft.body });
    } catch (e) {
      toast.error(String(e));
    }
  };

  const readyRows = rows.filter((r) => r.status === "ready" && r.generatedEmailId);
  const sendable = readyRows.slice(0, SEND_CAP);

  const sendAll = async () => {
    if (!batch) return;
    setConfirmOpen(false);
    setSending(true);
    let sent = 0;
    let failed = 0;
    const remaining = [...sendable];
    for (const row of remaining) {
      if (!row.generatedEmailId) continue;
      try {
        await ipc.generatedEmail.setStatus(row.generatedEmailId, "approved");
        await ipc.generatedEmail.send(
          row.generatedEmailId,
          attachment ? attachment.path : null,
          false,
        );
        sent += 1;
      } catch (e) {
        failed += 1;
        toast.error(`${row.email}: ${String(e)}`);
      }
      setSendProgress({ done: sent + failed, total: remaining.length });
      if (remaining.indexOf(row) < remaining.length - 1) {
        await new Promise((resolve) => setTimeout(resolve, SEND_DELAY_MS));
      }
    }
    try {
      await ipc.bulk.batchFinish(batch.id, sent > 0 ? "sent" : "failed");
    } catch {
      // status is cosmetic; counts are already recorded
    }
    setSending(false);
    setSendProgress(null);
    toast.success(`Bulk run complete — ${sent} sent, ${failed} failed`);
    if (readyRows.length > SEND_CAP) {
      toast.message(`${readyRows.length - SEND_CAP} recipients remain — send again for the next 50.`);
    }
    navigate("/emails");
  };

  const statusChip = (status: string) => {
    switch (status) {
      case "ready":
        return <Badge>Ready</Badge>;
      case "invalid":
        return <Badge variant="destructive">Invalid email</Badge>;
      case "duplicate":
        return <Badge variant="destructive">Duplicate</Badge>;
      case "failed":
        return <Badge variant="destructive">Failed</Badge>;
      default:
        return <Badge variant="secondary">{status}</Badge>;
    }
  };

  return (
    <section>
      <header className="mb-8 flex items-center gap-3">
        <Layers className="h-6 w-6 text-primary" />
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Bulk outreach</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Import a spreadsheet, generate personalized drafts for every recipient, review, then
            send in paced batches — never automatic.
          </p>
        </div>
      </header>

      <div className="space-y-6">
        <SectionCard
          title="1. Import spreadsheet"
          description="CSV or XLSX with one recipient per row and a header row on top."
          action={
            step === "upload" ? (
              <Button onClick={() => void pickFile()}>
                <Upload /> Choose file
              </Button>
            ) : (
              <Button variant="outline" size="sm" onClick={() => void pickFile()}>
                Replace file
              </Button>
            )
          }
        >
          {preview ? (
            <div className="space-y-2 text-sm">
              <p>
                <span className="font-medium">{preview.totalDataRows}</span> data rows ·{" "}
                {preview.headers.length} columns
              </p>
              <div className="overflow-x-auto rounded-lg border border-border">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="border-b bg-muted/50">
                      {preview.headers.map((header) => (
                        <th key={header} className="px-2 py-1.5 text-left font-medium">
                          {header}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {preview.sampleRows.map((row, i) => (
                      <tr key={i} className="border-b last:border-0">
                        {row.map((cell, j) => (
                          <td key={j} className="max-w-40 truncate px-2 py-1.5">
                            {cell}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          ) : (
            <EmptyState
              icon={Upload}
              title="No file imported"
              description="Upload a CSV or XLSX exported from your spreadsheet."
            />
          )}
        </SectionCard>

        {step !== "upload" && preview ? (
          <SectionCard
            title="2. Map columns & configure"
            description="Which column holds what? Empty fields fall back to the values below."
            action={
              <Button onClick={() => void generate()} disabled={generating || !mapping.email}>
                <Sparkles /> {generating ? "Generating…" : "Create batch & generate"}
              </Button>
            }
          >
            <div className="grid gap-4 sm:grid-cols-2">
              {MAPPABLE_FIELDS.map(({ key, label, required }) => (
                <div key={key}>
                  <Label className="mb-1.5 text-xs">
                    {label}
                    {required ? <span className="text-destructive">*</span> : null}
                  </Label>
                  <Select
                    value={mapping[key]}
                    onChange={(e) => setMapping((prev) => ({ ...prev, [key]: e.target.value }))}
                  >
                    <option value="">Not mapped</option>
                    {preview.headers.map((header) => (
                      <option key={header} value={header}>
                        {header}
                      </option>
                    ))}
                  </Select>
                </div>
              ))}
              <div>
                <Label className="mb-1.5 text-xs">Email type</Label>
                <Select
                  value={emailType}
                  onChange={(e) => setEmailType(e.target.value as EmailType)}
                >
                  {EMAIL_TYPES.map((t) => (
                    <option key={t} value={t}>
                      {EMAIL_TYPE_LABELS[t]}
                    </option>
                  ))}
                </Select>
              </div>
              <div>
                <Label className="mb-1.5 text-xs">Link to application</Label>
                <Select
                  value={applicationId}
                  onChange={(e) => setApplicationId(e.target.value)}
                >
                  <option value="">None</option>
                  {applications.map((app) => (
                    <option key={app.id} value={app.id}>
                      {app.company} · {app.role}
                    </option>
                  ))}
                </Select>
              </div>
            </div>
          </SectionCard>
        ) : null}

        {step === "review" ? (
          <SectionCard
            title={`3. Review recipients${readyRows.length ? ` — ${readyRows.length} ready` : ""}`}
            description="Preview, remove, then send. Sends are paced 2 seconds apart, max 50 per run."
            action={
              <div className="flex items-center gap-2">
                {rows.some((r) => r.status === "failed") ? (
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={retrying || generating}
                    onClick={() => void retryFailed()}
                  >
                    <RefreshCw className={retrying ? "animate-spin" : undefined} />
                    {retrying ? "Retrying…" : `Retry failed (${rows.filter((r) => r.status === "failed").length})`}
                  </Button>
                ) : null}
                <Button
                  disabled={sending || sendable.length === 0}
                  onClick={() => setConfirmOpen(true)}
                >
                  <Send /> Send {sendable.length} email{sendable.length === 1 ? "" : "s"}
                </Button>
              </div>
            }
          >
            <div className="mb-4 flex flex-wrap items-center gap-2">
              <Label htmlFor="bulk-attach" className="text-xs">Attach resume</Label>
              {resumeFiles.length > 0 ? (
                <Select
                  id="bulk-attach"
                  className="h-7 w-56 text-xs"
                  value={attachment && resumeFiles.some((f) => f.storedPath === attachment.path) ? attachment.path : ""}
                  onChange={(e) => {
                    const file = resumeFiles.find((f) => f.storedPath === e.target.value);
                    setAttachment(file ? { path: file.storedPath, name: file.originalFilename } : null);
                  }}
                >
                  <option value="">No attachment</option>
                  {resumeFiles.map((file) => (
                    <option key={file.id} value={file.storedPath}>
                      {file.originalFilename}
                      {file.id === defaultResumeId ? " (default)" : ""}
                    </option>
                  ))}
                </Select>
              ) : (
                <span className="text-xs text-muted-foreground">
                  No master resumes uploaded — upload one on the Resumes page to attach.
                </span>
              )}
            </div>
            {sending && sendProgress ? (
              <div className="mb-4 space-y-1.5">
                <div className="flex justify-between text-xs text-muted-foreground">
                  <span>Sending…</span>
                  <span>
                    {sendProgress.done} / {sendProgress.total}
                  </span>
                </div>
                <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
                  <div
                    className="h-full rounded-full bg-primary transition-all"
                    style={{
                      width: `${Math.round((sendProgress.done / Math.max(sendProgress.total, 1)) * 100)}%`,
                    }}
                  />
                </div>
              </div>
            ) : null}
            {rows.length === 0 ? (
              <p className="text-sm text-muted-foreground">No rows generated.</p>
            ) : (
              <ul className="max-h-96 divide-y overflow-y-auto pr-1">
                {rows.map((row) => (
                  <li
                    key={row.rowIndex}
                    className="flex items-center gap-3 py-2.5 text-sm first:pt-0 last:pb-0"
                  >
                    <span className="w-48 min-w-0 shrink-0 truncate">
                      {row.name || "(no name)"}
                      <span className="ml-1.5 text-xs text-muted-foreground">{row.email}</span>
                    </span>
                    {statusChip(row.status)}
                    {row.detail ? (
                      <span className="min-w-0 flex-1 truncate text-xs text-destructive">
                        {row.detail}
                      </span>
                    ) : (
                      <span className="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                        {[row.company, row.role].filter(Boolean).join(" · ")}
                      </span>
                    )}
                    {row.generatedEmailId ? (
                      <>
                        <Button
                          variant="ghost"
                          size="xs"
                          onClick={() => void previewRecipient(row.generatedEmailId!)}
                        >
                          Preview
                        </Button>
                        {row.status === "ready" && !sending ? (
                          <DeleteButton
                            confirmLabel="Remove"
                            cancelLabel="Keep"
                            onConfirm={() => removeRecipient(row)}
                          />
                        ) : null}
                      </>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
          </SectionCard>
        ) : null}
      </div>

      {previewDraft ? (
        <Dialog open onOpenChange={(o) => !o && setPreviewDraft(null)}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{previewDraft.subject}</DialogTitle>
              <DialogDescription>Generated draft preview</DialogDescription>
            </DialogHeader>
            <pre className="max-h-72 overflow-y-auto whitespace-pre-wrap rounded-md bg-muted p-3 text-xs">
              {previewDraft.body}
            </pre>
            <DialogFooter>
              <Button variant="outline" onClick={() => setPreviewDraft(null)}>
                Close
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      ) : null}

      {confirmOpen && batch ? (
        <Dialog open onOpenChange={(o) => !o && setConfirmOpen(false)}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Send {sendable.length} emails?</DialogTitle>
              <DialogDescription>
                They will be sent one by one, 2 seconds apart, to the exact recipients listed
                above. This cannot be undone.
              </DialogDescription>
              {attachment ? (
                <p className="text-xs text-muted-foreground">
                  Resume attached: <span className="font-medium">{attachment.name}</span>
                </p>
              ) : (
                <p className="text-xs text-muted-foreground">No resume will be attached.</p>
              )}
            </DialogHeader>
            <DialogFooter>
              <Button variant="outline" onClick={() => setConfirmOpen(false)} disabled={sending}>
                Cancel
              </Button>
              <Button onClick={() => void sendAll()} disabled={sending}>
                <Send /> {sending ? "Sending…" : "Confirm & send"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      ) : null}
    </section>
  );
}
