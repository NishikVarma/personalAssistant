import { useEffect, useState } from "react";
import { Pencil, Plus } from "lucide-react";
import DeleteButton from "@/components/profile/DeleteButton";
import FormDialog, { emptyToNull, type FormValues } from "@/components/profile/FormDialog";
import SectionCard from "@/components/profile/SectionCard";
import VerifiedToggle from "@/components/profile/VerifiedToggle";
import { Button } from "@/components/ui/button";
import { ipc, type Education, type EducationInput } from "@/lib/ipc";

const FIELDS = [
  { name: "institution", label: "Institution", type: "text" as const, required: true },
  { name: "degree", label: "Degree", type: "text" as const, placeholder: "B.Tech" },
  {
    name: "fieldOfStudy",
    label: "Field of study",
    type: "text" as const,
    placeholder: "Computer Science",
  },
  { name: "grade", label: "Grade", type: "text" as const, placeholder: "8.9 CGPA" },
  { name: "startDate", label: "Start date", type: "date" as const },
  { name: "endDate", label: "End date", type: "date" as const },
  { name: "location", label: "Location", type: "text" as const },
  {
    name: "details",
    label: "Details",
    type: "textarea" as const,
    placeholder: "Minor, honors, relevant coursework…",
  },
];

function toInitial(item: Education): FormValues {
  return {
    institution: item.institution,
    degree: item.degree,
    fieldOfStudy: item.fieldOfStudy,
    grade: item.grade ?? "",
    startDate: item.startDate ?? "",
    endDate: item.endDate ?? "",
    location: item.location ?? "",
    details: item.details,
  };
}

function toInput(values: FormValues): EducationInput {
  return {
    institution: values.institution.trim(),
    degree: values.degree.trim(),
    fieldOfStudy: values.fieldOfStudy.trim(),
    grade: emptyToNull(values.grade),
    startDate: emptyToNull(values.startDate),
    endDate: emptyToNull(values.endDate),
    location: emptyToNull(values.location),
    details: values.details.trim(),
  };
}

export default function EducationSection() {
  const [items, setItems] = useState<Education[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [dialog, setDialog] = useState<
    | { mode: "add" }
    | { mode: "edit"; item: Education }
    | null
  >(null);

  const reload = () => {
    ipc.education
      .list()
      .then(setItems)
      .catch((e) => setError(String(e)));
  };

  useEffect(reload, []);

  return (
    <SectionCard
      title="Education"
      description="Degrees and programs the AI may reference in applications."
      action={
        <Button variant="outline" size="sm" onClick={() => setDialog({ mode: "add" })}>
          <Plus /> Add
        </Button>
      }
    >
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {items.length === 0 && !error ? (
        <p className="text-sm text-muted-foreground">No education added yet.</p>
      ) : (
        <ul className="divide-y">
          {items.map((item) => (
            <li key={item.id} className="group flex items-center gap-3 py-3 first:pt-0 last:pb-0">
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium">{item.institution}</p>
                <p className="truncate text-sm text-muted-foreground">
                  {[item.degree, item.fieldOfStudy].filter(Boolean).join(" · ")}
                  {item.startDate || item.endDate
                    ? ` · ${item.startDate ?? "?"} – ${item.endDate ?? "present"}`
                    : ""}
                  {item.grade ? ` · ${item.grade}` : ""}
                </p>
                {item.details ? (
                  <p className="mt-0.5 truncate text-xs text-muted-foreground">{item.details}</p>
                ) : null}
              </div>
              <VerifiedToggle
                verified={item.verified}
                onToggle={async () => {
                  await ipc.education.setVerified(item.id, !item.verified);
                  reload();
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
                  await ipc.education.remove(item.id);
                  reload();
                }}
              />
            </li>
          ))}
        </ul>
      )}

      {dialog ? (
        <FormDialog
          title={dialog.mode === "add" ? "Add education" : "Edit education"}
          fields={FIELDS}
          initial={dialog.mode === "edit" ? toInitial(dialog.item) : {}}
          onSubmit={async (values) => {
            if (dialog.mode === "add") {
              await ipc.education.create(toInput(values));
            } else {
              await ipc.education.update(dialog.item.id, toInput(values));
            }
            reload();
          }}
          onClose={() => setDialog(null)}
        />
      ) : null}
    </SectionCard>
  );
}
