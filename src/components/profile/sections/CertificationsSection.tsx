import { useEffect, useState } from "react";
import { Pencil, Plus } from "lucide-react";
import DeleteButton from "@/components/profile/DeleteButton";
import FormDialog, { emptyToNull, type FormValues } from "@/components/profile/FormDialog";
import { shortDate } from "@/components/profile/labels";
import SectionCard from "@/components/profile/SectionCard";
import VerifiedToggle from "@/components/profile/VerifiedToggle";
import { Button } from "@/components/ui/button";
import { ipc, type Certification, type CertificationInput } from "@/lib/ipc";

const FIELDS = [
  { name: "name", label: "Certification", type: "text" as const, required: true },
  { name: "issuer", label: "Issuer", type: "text" as const },
  { name: "issueDate", label: "Issue date", type: "date" as const },
  { name: "expiryDate", label: "Expiry date", type: "date" as const },
  {
    name: "credentialUrl",
    label: "Credential URL",
    type: "text" as const,
    full: true,
    placeholder: "https://…",
  },
];

function toInitial(item: Certification): FormValues {
  return {
    name: item.name,
    issuer: item.issuer,
    issueDate: item.issueDate ?? "",
    expiryDate: item.expiryDate ?? "",
    credentialUrl: item.credentialUrl ?? "",
  };
}

function toInput(values: FormValues): CertificationInput {
  return {
    name: values.name.trim(),
    issuer: values.issuer.trim(),
    issueDate: emptyToNull(values.issueDate),
    expiryDate: emptyToNull(values.expiryDate),
    credentialUrl: emptyToNull(values.credentialUrl),
  };
}

export default function CertificationsSection() {
  const [items, setItems] = useState<Certification[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [dialog, setDialog] = useState<
    { mode: "add" } | { mode: "edit"; item: Certification } | null
  >(null);

  const reload = () => {
    ipc.certification
      .list()
      .then(setItems)
      .catch((e) => setError(String(e)));
  };

  useEffect(reload, []);

  return (
    <SectionCard
      title="Certifications"
      description="Credentials the AI may cite; verify only what is real."
      action={
        <Button variant="outline" size="sm" onClick={() => setDialog({ mode: "add" })}>
          <Plus /> Add
        </Button>
      }
    >
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {items.length === 0 && !error ? (
        <p className="text-sm text-muted-foreground">No certifications added yet.</p>
      ) : (
        <ul className="divide-y">
          {items.map((item) => (
            <li key={item.id} className="flex items-center gap-3 py-3 first:pt-0 last:pb-0">
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium">{item.name}</p>
                <p className="truncate text-sm text-muted-foreground">
                  {[
                    item.issuer,
                    [shortDate(item.issueDate), shortDate(item.expiryDate)].filter(Boolean).join(" – "),
                    item.credentialUrl,
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </p>
              </div>
              <VerifiedToggle
                verified={item.verified}
                onToggle={async () => {
                  await ipc.certification.setVerified(item.id, !item.verified);
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
                  await ipc.certification.remove(item.id);
                  reload();
                }}
              />
            </li>
          ))}
        </ul>
      )}

      {dialog ? (
        <FormDialog
          title={dialog.mode === "add" ? "Add certification" : "Edit certification"}
          fields={FIELDS}
          initial={dialog.mode === "edit" ? toInitial(dialog.item) : {}}
          onSubmit={async (values) => {
            if (dialog.mode === "add") {
              await ipc.certification.create(toInput(values));
            } else {
              await ipc.certification.update(dialog.item.id, toInput(values));
            }
            reload();
          }}
          onClose={() => setDialog(null)}
        />
      ) : null}
    </SectionCard>
  );
}
