import { useEffect, useState } from "react";
import { Pencil, Plus } from "lucide-react";
import { toast } from "sonner";
import DeleteButton from "@/components/profile/DeleteButton";
import EntityAttachments from "@/components/profile/EntityAttachments";
import FormDialog, { emptyToNull, type FormValues } from "@/components/profile/FormDialog";
import { EMPLOYMENT_TYPE_OPTIONS, labelFor, shortDate } from "@/components/profile/labels";
import SectionCard from "@/components/profile/SectionCard";
import VerifiedToggle from "@/components/profile/VerifiedToggle";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ipc, type Experience, type ExperienceInput, type EmploymentType } from "@/lib/ipc";

const FIELDS = [
  { name: "organization", label: "Organization", type: "text" as const, required: true },
  { name: "title", label: "Title", type: "text" as const, required: true },
  {
    name: "employmentType",
    label: "Employment type",
    type: "select" as const,
    options: EMPLOYMENT_TYPE_OPTIONS,
  },
  { name: "location", label: "Location", type: "text" as const },
  { name: "startDate", label: "Start date", type: "date" as const },
  { name: "endDate", label: "End date", type: "date" as const },
  {
    name: "currentlyWorking",
    label: "Currently working here",
    type: "select" as const,
    options: [
      { value: "no", label: "No" },
      { value: "yes", label: "Yes" },
    ],
  },
  {
    name: "description",
    label: "Description",
    type: "textarea" as const,
    placeholder: "What you worked on. Add verified resume bullets below after saving.",
  },
];

function toInitial(item: Experience): FormValues {
  return {
    organization: item.organization,
    title: item.title,
    employmentType: item.employmentType,
    location: item.location ?? "",
    startDate: item.startDate ?? "",
    endDate: item.endDate ?? "",
    currentlyWorking: item.currentlyWorking ? "yes" : "no",
    description: item.description,
  };
}

function toInput(values: FormValues): ExperienceInput {
  return {
    organization: values.organization.trim(),
    title: values.title.trim(),
    employmentType: values.employmentType as EmploymentType,
    location: emptyToNull(values.location),
    startDate: emptyToNull(values.startDate),
    endDate: emptyToNull(values.endDate),
    currentlyWorking: values.currentlyWorking === "yes",
    description: values.description.trim(),
  };
}

export default function ExperienceSection() {
  const [items, setItems] = useState<Experience[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [dialog, setDialog] = useState<{ mode: "add" } | { mode: "edit"; item: Experience } | null>(
    null,
  );

  const reload = () => {
    ipc.experience
      .list()
      .then(setItems)
      .catch((e) => setError(String(e)));
  };

  useEffect(reload, []);

  return (
    <SectionCard
      title="Experience"
      description="Internships and jobs with their verified resume bullets."
      action={
        <Button variant="outline" size="sm" onClick={() => setDialog({ mode: "add" })}>
          <Plus /> Add
        </Button>
      }
    >
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {items.length === 0 && !error ? (
        <p className="text-sm text-muted-foreground">No experience added yet.</p>
      ) : (
        <ul className="divide-y">
          {items.map((item) => (
            <li key={item.id} className="py-4 first:pt-0 last:pb-0">
              <div className="flex items-start gap-3">
                <div className="min-w-0 flex-1">
                  <p className="flex flex-wrap items-center gap-2 text-sm font-medium">
                    {item.organization}
                    <span className="font-normal text-muted-foreground">·</span>
                    <span className="font-normal">{item.title}</span>
                    <Badge variant="outline">
                      {labelFor(EMPLOYMENT_TYPE_OPTIONS, item.employmentType)}
                    </Badge>
                  </p>
                  <p className="text-sm text-muted-foreground">
                    {[shortDate(item.startDate), item.currentlyWorking ? "present" : shortDate(item.endDate)]
                      .filter(Boolean)
                      .join(" – ") || item.location || ""}
                    {!item.currentlyWorking && item.location ? ` · ${item.location}` : ""}
                  </p>
                  {item.description ? (
                    <p className="mt-1 text-sm">{item.description}</p>
                  ) : null}
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  <VerifiedToggle
                    verified={item.verified}
                    onToggle={async () => {
                      try {
                        await ipc.experience.setVerified(item.id, !item.verified);
                        toast.success(item.verified ? "Marked unverified" : "Marked verified");
                        reload();
                      } catch (e) {
                        toast.error(String(e));
                      }
                    }}
                  />
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    aria-label="Edit"
                    onClick={() => setDialog({ mode: "edit", item })}
                  >
                    <Pencil />
                  </Button>
                  <DeleteButton
                    onConfirm={async () => {
                      try {
                        await ipc.experience.remove(item.id);
                        toast.success("Experience deleted (bullets and skill links cleaned up)");
                        reload();
                      } catch (e) {
                        toast.error(String(e));
                      }
                    }}
                  />
                </div>
              </div>
              <EntityAttachments entityType="experience" entityId={item.id} />
            </li>
          ))}
        </ul>
      )}

      {dialog ? (
        <FormDialog
          title={dialog.mode === "add" ? "Add experience" : "Edit experience"}
          fields={FIELDS}
          initial={
            dialog.mode === "edit"
              ? toInitial(dialog.item)
              : { employmentType: "internship", currentlyWorking: "no" }
          }
          onSubmit={async (values) => {
            if (dialog.mode === "add") {
              await ipc.experience.create(toInput(values));
              toast.success("Experience added");
            } else {
              await ipc.experience.update(dialog.item.id, toInput(values));
              toast.success("Experience updated");
            }
            reload();
          }}
          onClose={() => setDialog(null)}
        />
      ) : null}
    </SectionCard>
  );
}
