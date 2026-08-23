import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Select } from "@/components/ui/select";
import { ipc, type Tag } from "@/lib/ipc";

interface ContactTagsEditorProps {
  contactId: number;
}

export default function ContactTagsEditor({ contactId }: ContactTagsEditorProps) {
  const [all, setAll] = useState<Tag[]>([]);
  const [linked, setLinked] = useState<Tag[]>([]);
  const [picked, setPicked] = useState("");
  const [busy, setBusy] = useState(false);

  const reload = () => {
    ipc.contact
      .listTags(contactId)
      .then(setLinked)
      .catch(() => setLinked([]));
    ipc.tag.list().then(setAll).catch(() => setAll([]));
  };

  useEffect(reload, [contactId]);

  const linkedIds = new Set(linked.map((t) => t.id));
  const available = all.filter((t) => !linkedIds.has(t.id));

  const save = async (tagIds: number[]) => {
    setBusy(true);
    try {
      await ipc.contact.replaceTags(contactId, tagIds);
      reload();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {linked.map((tag) => (
        <Badge key={tag.id} variant="secondary" className="gap-0.5 pr-1">
          {tag.name}
          <button
            type="button"
            aria-label={`Remove ${tag.name}`}
            disabled={busy}
            className="rounded-full p-0.5 hover:bg-foreground/10"
            onClick={() => save(linked.filter((t) => t.id !== tag.id).map((t) => t.id))}
          >
            <X className="size-3" />
          </button>
        </Badge>
      ))}
      {available.length > 0 ? (
        <Select
          className="h-6 w-32 text-xs"
          value={picked}
          disabled={busy}
          onChange={(e) => {
            const id = Number(e.target.value);
            if (id) void save([...linked.map((t) => t.id), id]);
            setPicked("");
          }}
        >
          <option value="">+ Add tag…</option>
          {available.map((tag) => (
            <option key={tag.id} value={tag.id}>
              {tag.name}
            </option>
          ))}
        </Select>
      ) : null}
      {all.length === 0 ? (
        <span className="text-xs text-muted-foreground">Create tags above first.</span>
      ) : null}
    </div>
  );
}
