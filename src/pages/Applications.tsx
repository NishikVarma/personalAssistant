import { useEffect, useState } from "react";
import { Pencil, Plus } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import DeleteButton from "@/components/profile/DeleteButton";
import FormDialog, { emptyToNull, type FormValues } from "@/components/profile/FormDialog";
import SectionCard from "@/components/profile/SectionCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
} from "@/components/ui/select";
import {
  APPLICATION_STATUSES,
  ipc,
  type Application,
  type ApplicationInput,
  type ApplicationStatus,
} from "@/lib/ipc";

const STATUS_LABELS: Record<ApplicationStatus, string> = {
  saved: "Saved",
  preparing: "Preparing",
  applied: "Applied",
  contacted: "Contacted",
  follow_up_due: "Follow-up due",
  response_received: "Response received",
  oa: "OA",
  interview: "Interview",
  offer: "Offer",
  rejected: "Rejected",
  withdrawn: "Withdrawn",
};

const FIELDS = [
  { name: "company", label: "Company", type: "text" as const, required: true },
  { name: "role", label: "Role", type: "text" as const, required: true },
  { name: "jobUrl", label: "Job URL", type: "text" as const, placeholder: "https://…" },
  { name: "source", label: "Source", type: "text" as const, placeholder: "referral, LinkedIn…" },
  { name: "dateDiscovered", label: "Date discovered", type: "date" as const },
  { name: "dateApplied", label: "Date applied", type: "date" as const },
  { name: "followUpDate", label: "Follow-up date", type: "date" as const },
  { name: "priority", label: "Priority (0-3)", type: "text" as const },
  {
    name: "jobDescription",
    label: "Job description",
    type: "textarea" as const,
    placeholder: "Paste the full job description here — used for resume matching later.",
  },
  { name: "notes", label: "Notes", type: "textarea" as const },
];

function toInitial(app: Application): FormValues {
  return {
    company: app.company,
    role: app.role,
    jobUrl: app.jobUrl ?? "",
    source: app.source ?? "",
    dateDiscovered: app.dateDiscovered ?? "",
    dateApplied: app.dateApplied ?? "",
    followUpDate: app.followUpDate ?? "",
    priority: String(app.priority),
    jobDescription: app.jobDescription,
    notes: app.notes,
  };
}

function toInput(values: FormValues): ApplicationInput {
  return {
    company: values.company.trim(),
    role: values.role.trim(),
    jobDescription: values.jobDescription.trim(),
    jobUrl: emptyToNull(values.jobUrl),
    source: emptyToNull(values.source),
    dateDiscovered: emptyToNull(values.dateDiscovered),
    dateApplied: emptyToNull(values.dateApplied),
    followUpDate: emptyToNull(values.followUpDate),
    interviewStatus: null,
    priority: Number(values.priority) || 0,
    notes: values.notes.trim(),
  };
}

function shortDate(value: string | null): string {
  if (!value) return "";
  return value.slice(0, 10);
}

export default function Applications() {
  const [apps, setApps] = useState<Application[]>([]);
  const [filter, setFilter] = useState<string>("all");
  const [error, setError] = useState<string | null>(null);
  const [dialog, setDialog] = useState<{ mode: "add" } | { mode: "edit"; item: Application } | null>(
    null,
  );

  const reload = (statusFilter = filter) => {
    ipc.application
      .list(statusFilter === "all" ? null : (statusFilter as ApplicationStatus))
      .then(setApps)
      .catch((e) => setError(String(e)));
  };

  useEffect(() => {
    reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <section>
      <header className="mb-8 flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Applications</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Every opportunity with its full context — status changes are always made by you.
          </p>
        </div>
        <Button onClick={() => setDialog({ mode: "add" })}>
          <Plus /> Add application
        </Button>
      </header>

      <SectionCard
        title={`All applications${apps.length ? ` (${apps.length})` : ""}`}
        action={
          <Select
            className="h-8 w-44"
            value={filter}
            onChange={(e) => {
              setFilter(e.target.value);
              reload(e.target.value);
            }}
          >
            <option value="all">All statuses</option>
            {APPLICATION_STATUSES.map((status) => (
              <option key={status} value={status}>
                {STATUS_LABELS[status]}
              </option>
            ))}
          </Select>
        }
      >
        {error ? <p className="mb-2 text-sm text-destructive">{error}</p> : null}
        {apps.length === 0 ? (
          <p className="text-sm text-muted-foreground">No applications yet.</p>
        ) : (
          <ul className="divide-y">
            {apps.map((app) => (
              <li key={app.id} className="py-4 first:pt-0 last:pb-0">
                <div className="flex items-start gap-3">
                  <div className="min-w-0 flex-1">
                    <p className="flex flex-wrap items-baseline gap-2 text-sm font-medium">
                      {app.company}
                      <span className="font-normal text-muted-foreground">·</span>
                      <span className="font-normal">{app.role}</span>
                      {app.priority > 0 ? (
                        <Badge variant="secondary">P{app.priority}</Badge>
                      ) : null}
                      {app.source ? (
                        <span className="text-xs font-normal text-muted-foreground">
                          via {app.source}
                        </span>
                      ) : null}
                    </p>
                    <p className="text-sm text-muted-foreground">
                      {[
                        app.dateDiscovered ? `found ${shortDate(app.dateDiscovered)}` : "",
                        app.dateApplied ? `applied ${shortDate(app.dateApplied)}` : "",
                        app.followUpDate ? `follow up ${shortDate(app.followUpDate)}` : "",
                      ]
                        .filter(Boolean)
                        .join(" · ") || "no dates recorded"}
                    </p>
                    {app.notes ? (
                      <p className="mt-0.5 truncate text-xs text-muted-foreground">{app.notes}</p>
                    ) : null}
                  </div>
                  <div className="flex shrink-0 items-center gap-1.5">
                    <Select
                      className="h-7 w-36 text-xs"
                      value={app.status}
                      onChange={(e) => {
                        void ipc.application
                          .setStatus(app.id, e.target.value as ApplicationStatus)
                          .then(() => reload());
                      }}
                    >
                      {APPLICATION_STATUSES.map((status) => (
                        <option key={status} value={status}>
                          {STATUS_LABELS[status]}
                        </option>
                      ))}
                    </Select>
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      aria-label="Edit"
                      onClick={() => setDialog({ mode: "edit", item: app })}
                    >
                      <Pencil />
                    </Button>
                    <DeleteButton
                      onConfirm={async () => {
                        await ipc.application.remove(app.id);
                        reload();
                      }}
                    />
                  </div>
                </div>
                {app.jobUrl ? (
                  <button
                    type="button"
                    className="mt-1 block truncate text-xs text-primary underline-offset-2 hover:underline"
                    onClick={() => void openUrl(app.jobUrl!)}
                  >
                    {app.jobUrl}
                  </button>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </SectionCard>

      {dialog ? (
        <FormDialog
          title={dialog.mode === "add" ? "Add application" : `Edit ${dialog.item.company}`}
          fields={FIELDS}
          initial={
            dialog.mode === "edit"
              ? toInitial(dialog.item)
              : { dateDiscovered: new Date().toISOString().slice(0, 10), priority: "0" }
          }
          onSubmit={async (values) => {
            if (dialog.mode === "add") {
              await ipc.application.create(toInput(values));
            } else {
              await ipc.application.update(dialog.item.id, toInput(values));
            }
            reload();
          }}
          onClose={() => setDialog(null)}
        />
      ) : null}
    </section>
  );
}
