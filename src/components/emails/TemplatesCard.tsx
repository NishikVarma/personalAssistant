import { useEffect, useState } from "react";
import { Pencil, Plus } from "lucide-react";
import { toast } from "sonner";
import DeleteButton from "@/components/profile/DeleteButton";
import EmptyState from "@/components/EmptyState";
import FormDialog, { emptyToNull, type FieldDef, type FormValues } from "@/components/profile/FormDialog";
import SectionCard from "@/components/profile/SectionCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { EMAIL_TYPES, ipc, type EmailTemplate, type EmailTemplateInput, type EmailType } from "@/lib/ipc";

const FIELDS: FieldDef[] = [
  {
    name: "emailType",
    label: "Email type",
    type: "select",
    required: true,
    options: EMAIL_TYPES.map((t) => ({ value: t, label: t })),
  },
  { name: "role", label: "Role", type: "text", placeholder: "e.g. Backend Engineer" },
  { name: "companyOrIndustry", label: "Company / industry", type: "text", placeholder: "e.g. Acme, fintech…" },
  { name: "subjectTemplate", label: "Subject", type: "text", full: true },
  {
    name: "bodyTemplate",
    label: "Body",
    type: "textarea",
    required: true,
    placeholder: "The reusable email text. Keep names/specifics as placeholders to personalize at generation time.",
  },
];

function toInitial(t: EmailTemplate): FormValues {
  return {
    emailType: t.emailType,
    role: t.role ?? "",
    companyOrIndustry: t.companyOrIndustry ?? "",
    subjectTemplate: t.subjectTemplate ?? "",
    bodyTemplate: t.bodyTemplate,
  };
}

function toInput(values: FormValues): EmailTemplateInput {
  return {
    emailType: values.emailType as EmailType,
    role: emptyToNull(values.role),
    companyOrIndustry: emptyToNull(values.companyOrIndustry),
    subjectTemplate: emptyToNull(values.subjectTemplate),
    bodyTemplate: values.bodyTemplate,
  };
}

export default function TemplatesCard() {
  const [templates, setTemplates] = useState<EmailTemplate[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialog, setDialog] = useState<
    { mode: "add" } | { mode: "edit"; item: EmailTemplate }
    | null
  >(null);

  const reload = () => {
    ipc.emailTemplate
      .list()
      .then((rows) => setTemplates(Array.isArray(rows) ? rows : []))
      .catch((e) => toast.error(String(e)))
      .finally(() => setLoading(false));
  };

  useEffect(reload, []);

  return (
    <SectionCard
      title="Templates"
      description="Reusable emails the generator adapts instead of writing from scratch."
      action={
        <Button variant="outline" size="sm" onClick={() => setDialog({ mode: "add" })}>
          <Plus /> Add
        </Button>
      }
    >
      {loading ? (
        <div className="space-y-2">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-4/5" />
        </div>
      ) : templates.length === 0 ? (
        <EmptyState
          icon={Plus}
          title="No templates yet"
          description="Save an approved draft as a template, or add one manually — generation will reuse it."
        />
      ) : (
        <ul className="max-h-80 divide-y overflow-y-auto pr-1">
          {templates.map((t) => (
            <li key={t.id} className="group flex items-center gap-3 py-3 text-sm first:pt-0 last:pb-0">
              <div className="min-w-0 flex-1">
                <p className="flex flex-wrap items-center gap-2">
                  <Badge variant="outline">{t.emailType}</Badge>
                  <span className="truncate font-medium">
                    {t.subjectTemplate || "(no subject)"}
                  </span>
                  {t.source === "generated" ? <Badge variant="secondary">auto</Badge> : null}
                </p>
                <p className="truncate text-xs text-muted-foreground">
                  {[
                    [t.role, t.companyOrIndustry].filter(Boolean).join(" at "),
                    `used ${t.timesUsed}×`,
                    t.lastUsedAt ? `last ${new Date(t.lastUsedAt).toLocaleDateString()}` : "",
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </p>
              </div>
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label="Edit"
                onClick={() => setDialog({ mode: "edit", item: t })}
              >
                <Pencil />
              </Button>
              <DeleteButton
                onConfirm={async () => {
                  try {
                    await ipc.emailTemplate.remove(t.id);
                    toast.success("Template deleted");
                    reload();
                  } catch (e) {
                    toast.error(String(e));
                  }
                }}
              />
            </li>
          ))}
        </ul>
      )}

      {dialog ? (
        <FormDialog
          title={dialog.mode === "add" ? "Add template" : "Edit template"}
          fields={FIELDS}
          initial={
            dialog.mode === "edit" ? toInitial(dialog.item) : { emailType: "cold_outreach" }
          }
          onSubmit={async (values) => {
            if (dialog.mode === "add") {
              await ipc.emailTemplate.create(toInput(values));
              toast.success("Template added");
            } else {
              await ipc.emailTemplate.update(dialog.item.id, toInput(values));
              toast.success("Template updated");
            }
            reload();
          }}
          onClose={() => setDialog(null)}
        />
      ) : null}
    </SectionCard>
  );
}
