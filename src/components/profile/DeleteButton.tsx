import { useState } from "react";
import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";

interface DeleteButtonProps {
  onConfirm: () => Promise<void> | void;
  confirmLabel?: string;
  cancelLabel?: string;
}

/** Two-step delete: first click asks, second click confirms. Avoids native confirm() which is unreliable inside webviews. */
export default function DeleteButton({
  onConfirm,
  confirmLabel = "Delete",
  cancelLabel = "Keep",
}: DeleteButtonProps) {
  const [confirming, setConfirming] = useState(false);

  if (!confirming) {
    return (
      <Button
        variant="ghost"
        size="icon-xs"
        aria-label="Delete"
        onClick={() => setConfirming(true)}
      >
        <Trash2 />
      </Button>
    );
  }

  return (
    <span className="flex items-center gap-1">
      <Button
        variant="destructive"
        size="xs"
        onClick={() => {
          setConfirming(false);
          void onConfirm();
        }}
      >
        {confirmLabel}
      </Button>
      <Button variant="outline" size="xs" onClick={() => setConfirming(false)}>
        {cancelLabel}
      </Button>
    </span>
  );
}
