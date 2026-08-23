import { useEffect, useState } from "react";
import { Pencil, Plus } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import DeleteButton from "@/components/profile/DeleteButton";
import FormDialog, { type FormValues } from "@/components/profile/FormDialog";
import { LINK_KIND_OPTIONS, labelFor } from "@/components/profile/labels";
import SectionCard from "@/components/profile/SectionCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ipc, type Link, type LinkInput, type LinkKind } from "@/lib/ipc";

const FIELDS = [
  { name: "label", label: "Label", type: "text" as const, required: true },
  {
    name: "kind",
    label: "Kind",
    type: "select" as const,
    options: LINK_KIND_OPTIONS,
  },
  { name: "url", label: "URL", type: "text" as const, required: true, full: true, placeholder: "https://…" },
];

function toInitial(item: Link): FormValues {
  return { label: item.label, kind: item.kind, url: item.url };
}

function toInput(values: FormValues): LinkInput {
  return {
    label: values.label.trim(),
    url: values.url.trim(),
    kind: values.kind as LinkKind,
  };
}

export default function LinksSection() {
  const [items, setItems] = useState<Link[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [dialog, setDialog] = useState<{ mode: "add" } | { mode: "edit"; item: Link } | null>(null);

  const reload = () => {
    ipc.link
      .list()
      .then(setItems)
      .catch((e) => setError(String(e)));
  };

  useEffect(reload, []);

  return (
    <SectionCard
      title="Links"
      description="Profiles and portfolios referenced in applications."
      action={
        <Button variant="outline" size="sm" onClick={() => setDialog({ mode: "add" })}>
          <Plus /> Add
        </Button>
      }
    >
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {items.length === 0 && !error ? (
        <p className="text-sm text-muted-foreground">No links added yet.</p>
      ) : (
        <ul className="divide-y">
          {items.map((item) => (
            <li key={item.id} className="flex items-center gap-3 py-3 first:pt-0 last:pb-0">
              <div className="min-w-0 flex-1">
                <p className="flex items-center gap-2 text-sm font-medium">
                  {item.label}
                  <Badge variant="outline">{labelFor(LINK_KIND_OPTIONS, item.kind)}</Badge>
                </p>
                <button
                  type="button"
                  className="truncate text-sm text-muted-foreground underline-offset-2 hover:underline"
                  onClick={() => void openUrl(item.url)}
                >
                  {item.url}
                </button>
              </div>
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
                  await ipc.link.remove(item.id);
                  reload();
                }}
              />
            </li>
          ))}
        </ul>
      )}

      {dialog ? (
        <FormDialog
          title={dialog.mode === "add" ? "Add link" : "Edit link"}
          fields={FIELDS}
          initial={
            dialog.mode === "edit" ? toInitial(dialog.item) : { kind: "other" }
          }
          onSubmit={async (values) => {
            if (dialog.mode === "add") {
              await ipc.link.create(toInput(values));
            } else {
              await ipc.link.update(dialog.item.id, toInput(values));
            }
            reload();
          }}
          onClose={() => setDialog(null)}
        />
      ) : null}
    </SectionCard>
  );
}
