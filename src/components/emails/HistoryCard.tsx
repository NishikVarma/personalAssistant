import { useEffect, useMemo, useState } from "react";
import { ArrowDownLeft, ArrowUpRight, RefreshCw } from "lucide-react";
import { toast } from "sonner";
import EmptyState from "@/components/EmptyState";
import FormDialog, { emptyToNull, type FormValues } from "@/components/profile/FormDialog";
import SectionCard from "@/components/profile/SectionCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import {
  EMAIL_TYPES,
  ipc,
  type Application,
  type Contact,
  type EmailHistory,
  type EmailType,
  type HistoryFilter,
  type ResponseStatus,
} from "@/lib/ipc";

const RESPONSE_LABELS: Record<ResponseStatus, string> = {
  awaiting: "Awaiting",
  replied: "Replied",
  no_reply_needed: "No reply needed",
};

const LOG_FIELDS = [
  {
    name: "senderEmail",
    label: "From (sender email)",
    type: "text" as const,
    required: true,
    full: true,
    placeholder: "recruiter@company.com",
  },
  { name: "contactId", label: "Link to contact", type: "select" as const, full: true },
  { name: "applicationId", label: "Link to application", type: "select" as const, full: true },
  {
    name: "emailType",
    label: "Email type",
    type: "select" as const,
    options: [{ value: "", label: "Unknown" }, ...EMAIL_TYPES.map((t) => ({ value: t, label: t }))],
  },
  { name: "occurredAt", label: "Received on", type: "date" as const },
  { name: "subject", label: "Subject", type: "text" as const, full: true },
  { name: "body", label: "Message (or snippet)", type: "textarea" as const, required: true },
];

interface HistoryCardProps {
  applications: Application[];
  contacts: Contact[];
  gmailConnected: boolean;
}

export default function HistoryCard({ applications, contacts, gmailConnected }: HistoryCardProps) {
  const [history, setHistory] = useState<EmailHistory[]>([]);
  const [loading, setLoading] = useState(true);
  const [contactFilter, setContactFilter] = useState("");
  const [applicationFilter, setApplicationFilter] = useState("");
  const [syncing, setSyncing] = useState(false);
  const [logOpen, setLogOpen] = useState(false);

  const filter = useMemo<HistoryFilter>(
    () => ({
      contactId: contactFilter ? Number(contactFilter) : null,
      applicationId: applicationFilter ? Number(applicationFilter) : null,
      limit: 100,
    }),
    [contactFilter, applicationFilter],
  );

  const reload = () => {
    ipc.emailHistory
      .list(filter)
      .then((rows) => setHistory(Array.isArray(rows) ? rows : []))
      .catch((e) => toast.error(String(e)))
      .finally(() => setLoading(false));
  };

  useEffect(reload, [filter]);

  const contactName = (id: number | null) =>
    contacts.find((c) => c.id === id)?.name ?? contacts.find((c) => c.id === id)?.email ?? null;

  const syncReplies = async () => {
    setSyncing(true);
    try {
      const result = await ipc.gmail.syncReplies();
      if (result.repliesFound > 0) {
        toast.success(`Found ${result.repliesFound} new repl${result.repliesFound === 1 ? "y" : "ies"}`);
      } else {
        toast.message(`No replies yet (checked ${result.checked} threads)`);
      }
      reload();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setSyncing(false);
    }
  };

  const contactOptions = useMemo(
    () => [{ value: "", label: "All contacts" }, ...contacts.map((c) => ({ value: String(c.id), label: c.name || c.email }))],
    [contacts],
  );
  const applicationOptions = useMemo(
    () => [
      { value: "", label: "All applications" },
      ...applications.map((a) => ({ value: String(a.id), label: `${a.company} · ${a.role}` })),
    ],
    [applications],
  );

  return (
    <SectionCard
      title="History"
      description="Every sent and received email, with response tracking."
      action={
        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setLogOpen(true)}
          >
            Log received
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={syncing || !gmailConnected}
            title={gmailConnected ? "Check Gmail for replies" : "Connect Gmail in Settings first"}
            onClick={() => void syncReplies()}
          >
            <RefreshCw className={syncing ? "animate-spin" : undefined} /> Sync replies
          </Button>
        </div>
      }
    >
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <Select
          className="h-7 w-44 text-xs"
          value={contactFilter}
          onChange={(e) => setContactFilter(e.target.value)}
        >
          {contactOptions.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </Select>
        <Select
          className="h-7 w-52 text-xs"
          value={applicationFilter}
          onChange={(e) => setApplicationFilter(e.target.value)}
        >
          {applicationOptions.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </Select>
      </div>

      {loading ? (
        <div className="space-y-2">
          <Skeleton className="h-8 w-full" />
          <Skeleton className="h-8 w-5/6" />
        </div>
      ) : history.length === 0 ? (
        <EmptyState
          icon={ArrowUpRight}
          title="No email history yet"
          description="Sent emails appear here automatically; replies are found via Sync replies."
        />
      ) : (
        <ul className="divide-y">
          {history.map((row) => (
            <li key={row.id} className="flex items-center gap-3 py-2.5 text-sm first:pt-0 last:pb-0">
              {row.direction === "outgoing" ? (
                <ArrowUpRight className="h-4 w-4 shrink-0 text-emerald-600" />
              ) : (
                <ArrowDownLeft className="h-4 w-4 shrink-0 text-sky-600" />
              )}
              <span className="w-40 min-w-0 shrink-0 truncate text-xs text-muted-foreground">
                {row.recipientEmail ?? "(no address)"}
              </span>
              <span className="min-w-0 flex-1 truncate">
                {row.subject || "(no subject)"}
              </span>
              {row.emailType ? <Badge variant="outline">{row.emailType}</Badge> : null}
              {row.direction === "outgoing" ? (
                <Select
                  className="h-6 w-32 text-xs"
                  value={row.responseStatus ?? ""}
                  onChange={(e) => {
                    const status = (e.target.value || null) as ResponseStatus | null;
                    void ipc.emailHistory
                      .setResponse(row.id, status)
                      .then(() => reload())
                      .catch((err) => toast.error(String(err)));
                  }}
                >
                  <option value="">No status</option>
                  {(Object.keys(RESPONSE_LABELS) as ResponseStatus[]).map((status) => (
                    <option key={status} value={status}>
                      {RESPONSE_LABELS[status]}
                    </option>
                  ))}
                </Select>
              ) : (
                <Badge variant="secondary">received</Badge>
              )}
              <span className="w-20 shrink-0 text-right text-xs text-muted-foreground">
                {new Date(row.occurredAt).toLocaleDateString()}
              </span>
            </li>
          ))}
        </ul>
      )}

      {logOpen ? (
        <FormDialog
          title="Log a received email"
          fields={LOG_FIELDS.map((field) =>
            field.name === "contactId"
              ? { ...field, options: [{ value: "", label: "None" }, ...contacts.map((c) => ({ value: String(c.id), label: contactName(c.id) ?? c.email }))] }
              : field.name === "applicationId"
                ? { ...field, options: [{ value: "", label: "None" }, ...applicationOptions.slice(1)] }
                : field,
          )}
          initial={{}}
          onSubmit={async (values: FormValues) => {
            await ipc.emailHistory.recordIncoming({
              contactId: values.contactId ? Number(values.contactId) : null,
              applicationId: values.applicationId ? Number(values.applicationId) : null,
              senderEmail: values.senderEmail.trim(),
              emailType: (emptyToNull(values.emailType) as EmailType | null),
              subject: emptyToNull(values.subject),
              body: values.body,
              occurredAt: emptyToNull(values.occurredAt),
            });
            toast.success("Received email logged");
            reload();
          }}
          onClose={() => setLogOpen(false)}
        />
      ) : null}
    </SectionCard>
  );
}
