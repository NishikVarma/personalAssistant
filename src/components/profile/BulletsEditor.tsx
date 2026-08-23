import { useEffect, useState } from "react";
import { Check, Pencil, Plus, Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ipc, type Bullet, type ProfileEntityType } from "@/lib/ipc";

interface BulletsEditorProps {
  entityType: ProfileEntityType;
  entityId: number;
}

export default function BulletsEditor({ entityType, entityId }: BulletsEditorProps) {
  const [bullets, setBullets] = useState<Bullet[]>([]);
  const [draft, setDraft] = useState("");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editingDraft, setEditingDraft] = useState("");
  const [busy, setBusy] = useState(false);

  const reload = () => {
    ipc.bullet
      .listForEntity(entityType, entityId)
      .then(setBullets)
      .catch(() => setBullets([]));
  };

  useEffect(reload, [entityType, entityId]);

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await fn();
      reload();
    } finally {
      setBusy(false);
    }
  };

  const addBullet = async () => {
    const content = draft.trim();
    if (!content) return;
    await run(async () => {
      await ipc.bullet.create(entityType, entityId, {
        content,
        displayOrder: bullets.length,
      });
      setDraft("");
    });
  };

  return (
    <div className="space-y-2">
      <ul className="space-y-1">
        {bullets.map((bullet) => (
          <li key={bullet.id} className="group flex items-start gap-2 text-sm">
            <span className="mt-1.5 size-1 shrink-0 rounded-full bg-muted-foreground" />
            {editingId === bullet.id ? (
              <>
                <Input
                  className="h-7"
                  value={editingDraft}
                  autoFocus
                  onChange={(e) => setEditingDraft(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && editingDraft.trim()) {
                      void run(() =>
                        ipc.bullet.update(bullet.id, {
                          content: editingDraft.trim(),
                          displayOrder: bullet.displayOrder,
                        }),
                      ).then(() => setEditingId(null));
                    }
                    if (e.key === "Escape") setEditingId(null);
                  }}
                />
                <Button
                  variant="ghost"
                  size="icon-xs"
                  disabled={busy || !editingDraft.trim()}
                  onClick={() =>
                    run(() =>
                      ipc.bullet.update(bullet.id, {
                        content: editingDraft.trim(),
                        displayOrder: bullet.displayOrder,
                      }),
                    ).then(() => setEditingId(null))
                  }
                >
                  <Check />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-xs"
                  onClick={() => setEditingId(null)}
                >
                  <X />
                </Button>
              </>
            ) : (
              <>
                <span
                  className={
                    bullet.verified
                      ? "flex-1 font-medium"
                      : "flex-1 text-foreground/90"
                  }
                  title={bullet.verified ? "Verified bullet" : "Unverified bullet"}
                >
                  {bullet.content}
                </span>
                {bullet.verified ? (
                  <Check className="mt-0.5 size-3.5 shrink-0 text-emerald-600" aria-label="Verified" />
                ) : null}
                <Button
                  variant="ghost"
                  size="icon-xs"
                  className="opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
                  disabled={busy}
                  onClick={() =>
                    run(() => ipc.bullet.setVerified(bullet.id, !bullet.verified))
                  }
                  title={bullet.verified ? "Mark unverified" : "Mark verified"}
                >
                  <Check />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-xs"
                  className="opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
                  onClick={() => {
                    setEditingId(bullet.id);
                    setEditingDraft(bullet.content);
                  }}
                  title="Edit bullet"
                >
                  <Pencil />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-xs"
                  className="opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
                  disabled={busy}
                  onClick={() => run(() => ipc.bullet.remove(bullet.id))}
                  title="Delete bullet"
                >
                  <Trash2 />
                </Button>
              </>
            )}
          </li>
        ))}
      </ul>
      <div className="flex items-center gap-2">
        <Input
          className="h-7 max-w-md"
          placeholder="Add a resume bullet…"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void addBullet();
          }}
        />
        <Button variant="outline" size="xs" disabled={busy || !draft.trim()} onClick={addBullet}>
          <Plus /> Add
        </Button>
      </div>
    </div>
  );
}
