import { useEffect, useState } from "react";
import { Pencil, Plus, Trophy } from "lucide-react";
import { toast } from "sonner";
import DeleteButton from "@/components/profile/DeleteButton";
import EmptyState from "@/components/EmptyState";
import FormDialog, { emptyToNull, type FormValues } from "@/components/profile/FormDialog";
import { shortDate } from "@/components/profile/labels";
import SectionCard from "@/components/profile/SectionCard";
import VerifiedToggle from "@/components/profile/VerifiedToggle";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ipc, type Achievement, type AchievementInput } from "@/lib/ipc";

const FIELDS = [
  { name: "title", label: "Achievement", type: "text" as const, required: true, full: true },
  { name: "date", label: "Date", type: "date" as const },
  {
    name: "description",
    label: "Description",
    type: "textarea" as const,
    placeholder: "Context and scale (e.g. 'Top 5 of 40,000 teams').",
  },
];

function toInitial(item: Achievement): FormValues {
  return {
    title: item.title,
    date: item.date ?? "",
    description: item.description,
  };
}

function toInput(values: FormValues): AchievementInput {
  return {
    title: values.title.trim(),
    description: values.description.trim(),
    date: emptyToNull(values.date),
  };
}

export default function AchievementsSection() {
  const [items, setItems] = useState<Achievement[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [dialog, setDialog] = useState<
    { mode: "add" } | { mode: "edit"; item: Achievement } | null
  >(null);

  const reload = () => {
    ipc.achievement
      .list()
      .then(setItems)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  };

  useEffect(reload, []);

  return (
    <SectionCard
      title="Achievements"
      description="Competitions, awards and recognitions worth mentioning."
      action={
        <Button variant="outline" size="sm" onClick={() => setDialog({ mode: "add" })}>
          <Plus /> Add
        </Button>
      }
    >
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {loading ? (
        <div className="space-y-2">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-4/5" />
        </div>
      ) : items.length === 0 ? (
        <EmptyState
          icon={Trophy}
          title="No achievements added yet"
          description="Competitions, awards and recognitions worth mentioning."
        />
      ) : (
        <ul className="divide-y">
          {items.map((item) => (
            <li key={item.id} className="flex items-start gap-3 py-3 first:pt-0 last:pb-0">
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium">
                  {item.title}
                  {item.date ? (
                    <span className="ml-2 font-normal text-muted-foreground">
                      {shortDate(item.date)}
                    </span>
                  ) : null}
                </p>
                {item.description ? (
                  <p className="mt-0.5 text-sm text-muted-foreground">{item.description}</p>
                ) : null}
              </div>
              <VerifiedToggle
                verified={item.verified}
                onToggle={async () => {
                  try {
                    await ipc.achievement.setVerified(item.id, !item.verified);
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
                    await ipc.achievement.remove(item.id);
                    toast.success("Achievement deleted");
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
          title={dialog.mode === "add" ? "Add achievement" : "Edit achievement"}
          fields={FIELDS}
          initial={dialog.mode === "edit" ? toInitial(dialog.item) : {}}
          onSubmit={async (values) => {
            if (dialog.mode === "add") {
              await ipc.achievement.create(toInput(values));
              toast.success("Achievement added");
            } else {
              await ipc.achievement.update(dialog.item.id, toInput(values));
              toast.success("Achievement updated");
            }
            reload();
          }}
          onClose={() => setDialog(null)}
        />
      ) : null}
    </SectionCard>
  );
}
