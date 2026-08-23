import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  Select,
} from "@/components/ui/select";
import { ipc, type ProfileEntityType, type Skill } from "@/lib/ipc";

interface EntitySkillsEditorProps {
  entityType: ProfileEntityType;
  entityId: number;
}

export default function EntitySkillsEditor({ entityType, entityId }: EntitySkillsEditorProps) {
  const [all, setAll] = useState<Skill[]>([]);
  const [linked, setLinked] = useState<Skill[]>([]);
  const [picked, setPicked] = useState("");
  const [busy, setBusy] = useState(false);

  const reload = () => {
    ipc.skill
      .listForEntity(entityType, entityId)
      .then(setLinked)
      .catch(() => setLinked([]));
    ipc.skill.list().then(setAll).catch(() => setAll([]));
  };

  useEffect(reload, [entityType, entityId]);

  const linkedIds = new Set(linked.map((s) => s.id));
  const available = all.filter((s) => !linkedIds.has(s.id));

  const save = async (skillIds: number[]) => {
    setBusy(true);
    try {
      await ipc.skill.replaceForEntity(entityType, entityId, skillIds);
      reload();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {linked.map((skill) => (
        <Badge key={skill.id} variant="secondary" className="gap-0.5 pr-1">
          {skill.name}
          <button
            type="button"
            aria-label={`Remove ${skill.name}`}
            disabled={busy}
            className="rounded-full p-0.5 hover:bg-foreground/10"
            onClick={() =>
              save(linked.filter((s) => s.id !== skill.id).map((s) => s.id))
            }
          >
            <X className="size-3" />
          </button>
        </Badge>
      ))}
      {available.length > 0 ? (
        <Select
          className="h-6 w-36 text-xs"
          value={picked}
          disabled={busy}
          onChange={(e) => {
            const id = Number(e.target.value);
            if (id) void save([...linked.map((s) => s.id), id]);
            setPicked("");
          }}
        >
          <option value="">+ Add skill…</option>
          {available.map((skill) => (
            <option key={skill.id} value={skill.id}>
              {skill.name}
            </option>
          ))}
        </Select>
      ) : null}
      {all.length === 0 ? (
        <span className="text-xs text-muted-foreground">
          Add skills in the Skills section first.
        </span>
      ) : null}
    </div>
  );
}
