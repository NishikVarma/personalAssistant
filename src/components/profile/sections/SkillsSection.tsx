import { useEffect, useState } from "react";
import { Pencil, Plus, Wrench, X } from "lucide-react";
import { toast } from "sonner";
import EmptyState from "@/components/EmptyState";
import FormDialog, { type FieldDef, type FormValues } from "@/components/profile/FormDialog";
import { SKILL_CATEGORY_OPTIONS, labelFor } from "@/components/profile/labels";
import SectionCard from "@/components/profile/SectionCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { ipc, type Skill, type SkillCategory, type SkillInput } from "@/lib/ipc";

const FIELDS: FieldDef[] = [
  { name: "name", label: "Skill", type: "text", required: true },
  {
    name: "category",
    label: "Category",
    type: "select",
    options: SKILL_CATEGORY_OPTIONS,
  },
];

function toInput(values: FormValues): SkillInput {
  return {
    name: values.name.trim(),
    category: values.category as SkillCategory,
  };
}

export default function SkillsSection() {
  const [items, setItems] = useState<Skill[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [dialog, setDialog] = useState<{ mode: "add" } | { mode: "edit"; item: Skill } | null>(null);
  const [quickName, setQuickName] = useState("");
  const [quickCategory, setQuickCategory] = useState<SkillCategory>("language");
  const [busy, setBusy] = useState(false);

  const reload = () => {
    ipc.skill
      .list()
      .then(setItems)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  };

  useEffect(reload, []);

  const quickAdd = async () => {
    if (!quickName.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await ipc.skill.create({ name: quickName.trim(), category: quickCategory });
      toast.success(`Skill “${quickName.trim()}” added`);
      setQuickName("");
      reload();
    } catch (e) {
      toast.error(String(e));
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SectionCard
      title="Skills"
      description="Attach these to projects and experience; the AI only claims skills listed here."
      action={
        <Button variant="outline" size="sm" onClick={() => setDialog({ mode: "add" })}>
          <Plus /> Add
        </Button>
      }
    >
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <Input
          className="h-7 w-44"
          placeholder="Quick add skill…"
          value={quickName}
          onChange={(e) => setQuickName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void quickAdd();
          }}
        />
        <Select
          className="h-7 w-36 text-xs"
          value={quickCategory}
          onChange={(e) => setQuickCategory(e.target.value as SkillCategory)}
        >
          {SKILL_CATEGORY_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </Select>
        <Button variant="outline" size="xs" disabled={busy || !quickName.trim()} onClick={quickAdd}>
          <Plus /> Add
        </Button>
      </div>

      {error ? <p className="mb-2 text-sm text-destructive">{error}</p> : null}
      {loading ? (
        <div className="flex flex-wrap gap-1.5">
          {[0, 1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-6 w-20 rounded-4xl" />
          ))}
        </div>
      ) : items.length === 0 ? (
        <EmptyState
          icon={Wrench}
          title="No skills added yet"
          description="Attach these to projects and experience; the AI only claims skills listed here."
        />
      ) : (
        <div className="flex flex-wrap gap-1.5">
          {items.map((skill) => (
            <Badge
              key={skill.id}
              variant="secondary"
              className="group h-6 gap-1 pr-1"
            >
              {skill.name}
              <span className="text-[10px] font-normal text-muted-foreground">
                {labelFor(SKILL_CATEGORY_OPTIONS, skill.category)}
              </span>
              <button
                type="button"
                aria-label={`Edit ${skill.name}`}
                className="rounded-full p-0.5 hover:bg-foreground/10"
                onClick={() => setDialog({ mode: "edit", item: skill })}
              >
                <Pencil className="size-3" />
              </button>
              <button
                type="button"
                aria-label={`Remove ${skill.name}`}
                className="rounded-full p-0.5 hover:bg-foreground/10"
                onClick={async () => {
                  try {
                    await ipc.skill.remove(skill.id);
                    toast.success(`Skill “${skill.name}” removed`);
                    reload();
                  } catch (e) {
                    toast.error(String(e));
                  }
                }}
              >
                <X className="size-3" />
              </button>
            </Badge>
          ))}
        </div>
      )}

      {dialog ? (
        <FormDialog
          title={dialog.mode === "add" ? "Add skill" : `Edit “${dialog.item.name}”`}
          fields={FIELDS}
          initial={
            dialog.mode === "edit"
              ? { name: dialog.item.name, category: dialog.item.category }
              : { category: "language" }
          }
          onSubmit={async (values) => {
            if (dialog.mode === "add") {
              await ipc.skill.create(toInput(values));
              toast.success(`Skill “${toInput(values).name}” added`);
            } else {
              await ipc.skill.update(dialog.item.id, toInput(values));
              toast.success("Skill updated");
            }
            reload();
          }}
          onClose={() => setDialog(null)}
        />
      ) : null}
    </SectionCard>
  );
}
