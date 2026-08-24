import { useEffect, useState } from "react";
import { useLocation } from "react-router-dom";
import { Check, ClipboardCopy, FileOutput, Mail, Paperclip, Pencil, Send, Sparkles, Wand2 } from "lucide-react";
import { toast } from "sonner";
import { open } from "@tauri-apps/plugin-dialog";
import HistoryCard from "@/components/emails/HistoryCard";
import TemplatesCard from "@/components/emails/TemplatesCard";
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import { copyText } from "@/lib/clipboard";
import {
  EMAIL_TYPES,
  ipc,
  type Application,
  type Contact,
  type EmailStatus,
  type EmailType,
  type GeneratedEmail,
  type GmailStatus,
} from "@/lib/ipc";

const EMAIL_TYPE_LABELS: Record<EmailType, string> = {
  cold_outreach: "Cold outreach",
  job_application: "Job application",
  referral_request: "Referral request",
  follow_up: "Follow-up",
  internship_inquiry: "Internship inquiry",
  application_status: "Application status",
};

const STATUS_LABELS: Record<EmailStatus, string> = {
  draft: "Draft",
  edited: "Edited",
  approved: "Approved",
  sent: "Sent",
  discarded: "Discarded",
};

function formatWhen(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString();
}

interface ComposeState {
  recipientEmail: string;
  recipientName: string;
  company: string;
  role: string;
  jobDescription: string;
  additionalContext: string;
  emailType: EmailType;
  applicationId: string;
}

const EMPTY_COMPOSE: ComposeState = {
  recipientEmail: "",
  recipientName: "",
  company: "",
  role: "",
  jobDescription: "",
  additionalContext: "",
  emailType: "cold_outreach",
  applicationId: "",
};

export default function Emails() {
  const location = useLocation();
  const [compose, setCompose] = useState<ComposeState>(EMPTY_COMPOSE);
  const [applications, setApplications] = useState<Application[]>([]);
  const [contacts, setContacts] = useState<Contact[]>([]);
  const [emails, setEmails] = useState<GeneratedEmail[]>([]);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<GeneratedEmail | null>(null);
  const [subjectDraft, setSubjectDraft] = useState("");
  const [bodyDraft, setBodyDraft] = useState("");
  const [generating, setGenerating] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [gmailStatus, setGmailStatus] = useState<GmailStatus | null>(null);
  const [attachmentPath, setAttachmentPath] = useState<string | null>(null);
  const [attachmentName, setAttachmentName] = useState<string | null>(null);
  const [sendDialogOpen, setSendDialogOpen] = useState(false);
  const [sending, setSending] = useState(false);
  const [forceSend, setForceSend] = useState(false);
  const [sendWarning, setSendWarning] = useState<string | null>(null);

  const reloadList = () => {
    ipc.generatedEmail
      .list()
      .then(setEmails)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    reloadList();
    ipc.application
      .list()
      .then(setApplications)
      .catch(() => setApplications([]));
    ipc.contact
      .list()
      .then((rows) => setContacts(Array.isArray(rows) ? rows : []))
      .catch(() => setContacts([]));
    ipc.gmail
      .status()
      .then(setGmailStatus)
      .catch(() => setGmailStatus(null));
    // a follow-up draft created on the Follow-ups page lands here
    const draftId = (location.state as { draftId?: number } | null)?.draftId;
    if (draftId) {
      ipc.generatedEmail
        .get(draftId)
        .then(loadDraft)
        .catch(() => {});
      window.history.replaceState({}, "");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const setComposeField = (patch: Partial<ComposeState>) =>
    setCompose((prev) => ({ ...prev, ...patch }));

  const prefillFromAddress = async () => {
    const email = compose.recipientEmail.trim();
    if (!email.includes("@")) return;
    setBusy(true);
    setError(null);
    try {
      const guess = await ipc.ai.extractContact(email);
      setComposeField({
        recipientName: guess.name ?? compose.recipientName,
        company: guess.organization ?? compose.company,
      });
      toast.message("Best guess filled in", {
        description: "Verify the name and company before generating.",
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const generate = async () => {
    if (!compose.recipientEmail.trim().includes("@")) {
      setError("Enter a valid recipient email first.");
      return;
    }
    setGenerating(true);
    setError(null);
    setNotice(null);
    try {
      const created = await ipc.ai.generateEmail({
        recipientEmail: compose.recipientEmail.trim(),
        recipientName: compose.recipientName.trim() || null,
        company: compose.company.trim() || null,
        role: compose.role.trim() || null,
        jobDescription: compose.jobDescription.trim() || null,
        additionalContext: compose.additionalContext.trim() || null,
        emailType: compose.emailType,
        applicationId: compose.applicationId ? Number(compose.applicationId) : null,
        contactId: null,
      });
      loadDraft(created);
      reloadList();
      toast.success("Draft generated — review it below");
    } catch (e) {
      toast.error(String(e));
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  };

  const loadDraft = (email: GeneratedEmail) => {
    setSelected(email);
    setSubjectDraft(email.subject ?? "");
    setBodyDraft(email.body);
    setNotice(null);
    setAttachmentPath(null);
    setAttachmentName(null);
  };

  const dirty =
    selected !== null &&
    ((selected.subject ?? "") !== subjectDraft || selected.body !== bodyDraft);

  const saveChanges = async () => {
    if (!selected) return;
    setBusy(true);
    setError(null);
    try {
      const saved = await ipc.generatedEmail.update(
        selected.id,
        subjectDraft.trim() || null,
        bodyDraft,
      );
      setSelected(saved);
      reloadList();
      toast.success("Draft saved");
    } catch (e) {
      toast.error(String(e));
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const changeStatus = async (status: EmailStatus) => {
    if (!selected) return;
    setBusy(true);
    setError(null);
    try {
      // save pending edits together with the status change
      if (dirty) {
        const saved = await ipc.generatedEmail.update(
          selected.id,
          subjectDraft.trim() || null,
          bodyDraft,
        );
        setSelected(saved);
      }
      const updated = await ipc.generatedEmail.setStatus(selected.id, status);
      setSelected(updated);
      reloadList();
      if (status === "approved") {
        toast.success("Draft approved — ready to send");
      } else {
        toast.success(`Marked ${STATUS_LABELS[status].toLowerCase()}`);
      }
    } catch (e) {
      toast.error(String(e));
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const copyBody = async () => {
    if (!selected) return;
    const ok = await copyText(bodyDraft);
    if (ok) {
      toast.success("Copied to clipboard");
    } else {
      toast.error("Could not access the clipboard");
    }
  };

  const pickAttachment = async () => {
    const file = await open({ multiple: false, title: "Attach a file" });
    if (typeof file === "string") {
      setAttachmentPath(file);
      setAttachmentName(file.split(/[\\/]/).pop() ?? file);
      toast.message("Attachment ready", { description: file });
    }
  };

  const openSendDialog = () => {
    setForceSend(false);
    setSendWarning(null);
    setSendDialogOpen(true);
  };

  const sendEmail = async (force: boolean) => {
    if (!selected) return;
    setSending(true);
    try {
      const sent = await ipc.generatedEmail.send(selected.id, attachmentPath, force);
      setSelected(sent);
      setSendDialogOpen(false);
      setSendWarning(null);
      setForceSend(false);
      setAttachmentPath(null);
      setAttachmentName(null);
      reloadList();
      toast.success(`Email sent to ${sent.recipientEmail ?? "recipient"}`);
    } catch (e) {
      const message = String(e);
      if (message.toLowerCase().includes("recent outreach")) {
        setSendWarning(message);
      } else {
        toast.error(message);
        setSendDialogOpen(false);
      }
    } finally {
      setSending(false);
    }
  };

  return (
    <section>
      <header className="mb-8">
        <h1 className="text-2xl font-semibold tracking-tight">Emails</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Generate personalized drafts grounded in your verified career profile. Nothing is ever
          sent automatically.
        </p>
      </header>

      <div className="space-y-6">
        <SectionCard
          title="Compose"
          description="The AI writes using only your career profile and these details."
          action={
            <Button onClick={() => void generate()} disabled={generating}>
              {generating ? (
                <Sparkles className="animate-pulse" />
              ) : (
                <Wand2 />
              )}
              {generating ? "Generating…" : "Generate draft"}
            </Button>
          }
        >
          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <Label htmlFor="compose-recipient" className="mb-1.5">
                Recipient email *
              </Label>
              <Input
                id="compose-recipient"
                type="email"
                value={compose.recipientEmail}
                placeholder="hr@company.com"
                onChange={(e) => setComposeField({ recipientEmail: e.target.value })}
                onBlur={() => void prefillFromAddress()}
              />
            </div>
            <div>
              <Label htmlFor="compose-name" className="mb-1.5">
                Recipient name
              </Label>
              <div className="flex gap-2">
                <Input
                  id="compose-name"
                  value={compose.recipientName}
                  placeholder="Guess from address"
                  onChange={(e) => setComposeField({ recipientName: e.target.value })}
                />
                <Button
                  variant="outline"
                  size="sm"
                  disabled={busy || !compose.recipientEmail.includes("@")}
                  onClick={() => void prefillFromAddress()}
                >
                  <Pencil /> Guess
                </Button>
              </div>
            </div>
            <div>
              <Label htmlFor="compose-company" className="mb-1.5">
                Company
              </Label>
              <Input
                id="compose-company"
                value={compose.company}
                onChange={(e) => setComposeField({ company: e.target.value })}
              />
            </div>
            <div>
              <Label htmlFor="compose-role" className="mb-1.5">
                Role
              </Label>
              <Input
                id="compose-role"
                value={compose.role}
                onChange={(e) => setComposeField({ role: e.target.value })}
              />
            </div>
            <div>
              <Label htmlFor="compose-type" className="mb-1.5">
                Email type
              </Label>
              <Select
                id="compose-type"
                value={compose.emailType}
                onChange={(e) => setComposeField({ emailType: e.target.value as EmailType })}
              >
                {EMAIL_TYPES.map((t) => (
                  <option key={t} value={t}>
                    {EMAIL_TYPE_LABELS[t]}
                  </option>
                ))}
              </Select>
            </div>
            <div>
              <Label htmlFor="compose-app" className="mb-1.5">
                Link to application
              </Label>
              <Select
                id="compose-app"
                value={compose.applicationId}
                onChange={(e) => setComposeField({ applicationId: e.target.value })}
              >
                <option value="">None</option>
                {applications.map((app) => (
                  <option key={app.id} value={app.id}>
                    {app.company} · {app.role}
                  </option>
                ))}
              </Select>
            </div>
            <div className="sm:col-span-2">
              <Label htmlFor="compose-jd" className="mb-1.5">
                Job description
              </Label>
              <Textarea
                id="compose-jd"
                rows={5}
                value={compose.jobDescription}
                placeholder="Paste the posting here for a tailored application."
                onChange={(e) => setComposeField({ jobDescription: e.target.value })}
              />
            </div>
            <div className="sm:col-span-2">
              <Label htmlFor="compose-context" className="mb-1.5">
                Additional context
              </Label>
              <Textarea
                id="compose-context"
                rows={2}
                value={compose.additionalContext}
                placeholder="Anything else worth mentioning."
                onChange={(e) => setComposeField({ additionalContext: e.target.value })}
              />
            </div>
          </div>

          <div className="mt-3 space-y-1 text-xs text-muted-foreground">
            <p className="flex items-center gap-1.5">
              <Check className="h-3.5 w-3.5 text-emerald-600" />
              Verify the guessed name/company before generating.
            </p>
          </div>
          {notice ? <p className="mt-2 text-sm text-muted-foreground">{notice}</p> : null}
          {error ? <p className="mt-2 text-sm text-destructive">{error}</p> : null}
        </SectionCard>

        {selected ? (
          <SectionCard
            title="Draft editor"
            description={
              selected.provider
                ? `Generated by ${selected.provider} (${selected.model})`
                : "Created manually"
            }
            action={
              <Badge variant={selected.status === "approved" ? "default" : "secondary"}>
                {STATUS_LABELS[selected.status]}
              </Badge>
            }
          >
            <div className="space-y-4">
              <div>
                <Label htmlFor="draft-subject" className="mb-1.5">
                  Subject
                </Label>
                <Input
                  id="draft-subject"
                  value={subjectDraft}
                  onChange={(e) => setSubjectDraft(e.target.value)}
                />
              </div>
              <div>
                <Label htmlFor="draft-body" className="mb-1.5">
                  Body
                </Label>
                <Textarea
                  id="draft-body"
                  rows={14}
                  value={bodyDraft}
                  onChange={(e) => setBodyDraft(e.target.value)}
                />
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Button variant="outline" size="sm" onClick={() => void copyBody()}>
                  <ClipboardCopy /> Copy
                </Button>
                <Button size="sm" disabled={!dirty || busy} onClick={() => void saveChanges()}>
                  Save changes
                </Button>
                {selected.status === "sent" ? null : (
                  <>
                {!["approved", "sent", "discarded"].includes(selected.status) ? (
                  <Button
                    size="sm"
                    disabled={busy}
                    onClick={() => void changeStatus("approved")}
                  >
                    Approve
                  </Button>
                ) : null}
                {selected.status === "approved" ? (
                  <>
                    <Button variant="outline" size="sm" onClick={() => void pickAttachment()}>
                      <Paperclip /> {attachmentName ?? "Attach file"}
                    </Button>
                    <Button
                      size="sm"
                      disabled={busy || !gmailStatus?.connected}
                      title={
                        gmailStatus?.connected
                          ? "Send via Gmail"
                          : "Connect Gmail in Settings first"
                      }
                      onClick={openSendDialog}
                    >
                      <Send /> Send via Gmail
                    </Button>
                  </>
                ) : null}
                    <DeleteButton
                      confirmLabel="Discard draft"
                      cancelLabel="Keep"
                      onConfirm={async () => {
                        if (!selected) return;
                        try {
                          await ipc.generatedEmail.remove(selected.id);
                          toast.success("Draft deleted");
                          setSelected(null);
                          reloadList();
                        } catch (e) {
                          toast.error(String(e));
                        }
                      }}
                    />
                  </>
                )}
                {selected.status === "approved" || selected.status === "sent" ? (
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={busy}
                    onClick={async () => {
                      try {
                        await ipc.generatedEmail.saveAsTemplate(selected.id);
                        toast.success("Saved as a reusable template");
                      } catch (e) {
                        toast.error(String(e));
                      }
                    }}
                  >
                    <FileOutput /> Save as template
                  </Button>
                ) : null}
                {dirty ? (
                  <span className="text-xs text-muted-foreground">Unsaved edits</span>
                ) : null}
              </div>
            </div>
          </SectionCard>
        ) : null}

        <Dialog open={sendDialogOpen} onOpenChange={(o) => !o && setSendDialogOpen(false)}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Send this email?</DialogTitle>
              <DialogDescription>
                Review everything below — this sends for real via Gmail.
              </DialogDescription>
            </DialogHeader>
            {selected ? (
              <div className="space-y-2 text-sm">
                <p>
                  <span className="text-muted-foreground">To: </span>
                  <span className="font-medium">
                    {selected.recipientEmail ?? "linked contact's address"}
                  </span>
                </p>
                <p>
                  <span className="text-muted-foreground">Subject: </span>
                  {subjectDraft || "(no subject)"}
                </p>
                {attachmentName ? (
                  <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <Paperclip className="h-3.5 w-3.5" /> {attachmentName}
                  </p>
                ) : null}
                <pre className="max-h-56 overflow-y-auto whitespace-pre-wrap rounded-md bg-muted p-3 text-xs">
                  {bodyDraft}
                </pre>
                {sendWarning ? (
                  <div className="rounded-md border border-destructive/50 bg-destructive/10 p-3 text-xs">
                    <p className="text-destructive">{sendWarning}</p>
                    <label className="mt-2 flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={forceSend}
                        onChange={(e) => setForceSend(e.target.checked)}
                      />
                      Send anyway
                    </label>
                  </div>
                ) : null}
              </div>
            ) : null}
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => setSendDialogOpen(false)}
                disabled={sending}
              >
                Cancel
              </Button>
              <Button
                disabled={sending || (Boolean(sendWarning) && !forceSend)}
                onClick={() => void sendEmail(forceSend)}
              >
                {sending ? "Sending…" : "Send"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        <HistoryCard applications={applications} contacts={contacts} gmailConnected={Boolean(gmailStatus?.connected)} />
        <TemplatesCard />

        <SectionCard title={`Draft history${emails.length ? ` (${emails.length})` : ""}`}>
          {loading ? (
            <div className="space-y-2">
              <Skeleton className="h-8 w-full" />
              <Skeleton className="h-8 w-5/6" />
            </div>
          ) : emails.length === 0 ? (
            <EmptyState
              icon={Mail}
              title="No generated emails yet"
              description="Fill in the compose form above and let the AI draft your first email."
            />
          ) : (
            <ul className="divide-y">
              {emails.map((email) => (
                <li key={email.id}>
                  <button
                    type="button"
                    className="group flex w-full items-center gap-3 py-3 text-left first:pt-0 last:pb-0"
                    onClick={() => loadDraft(email)}
                  >
                    <Badge variant="outline">{EMAIL_TYPE_LABELS[email.emailType]}</Badge>
                    <span className="min-w-0 flex-1 truncate text-sm group-hover:underline">
                      {email.subject || "(no subject)"}
                    </span>
                    <Badge
                      variant={
                        email.status === "approved"
                          ? "default"
                          : email.status === "discarded"
                            ? "destructive"
                            : "secondary"
                      }
                    >
                      {STATUS_LABELS[email.status]}
                    </Badge>
                    <span className="shrink-0 text-xs text-muted-foreground">
                      {formatWhen(email.createdAt)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </SectionCard>
      </div>
    </section>
  );
}
