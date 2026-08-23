import { useState } from "react";
import { BadgeCheck, ShieldOff } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

interface VerifiedToggleProps {
  verified: boolean;
  onToggle: () => Promise<void> | void;
}

export default function VerifiedToggle({ verified, onToggle }: VerifiedToggleProps) {
  const [busy, setBusy] = useState(false);

  const handleClick = async () => {
    setBusy(true);
    try {
      await onToggle();
    } finally {
      setBusy(false);
    }
  };

  return (
    <Button
      variant="ghost"
      size="xs"
      className="-mx-1"
      disabled={busy}
      onClick={handleClick}
      title={verified ? "Mark as unverified" : "Mark as verified"}
    >
      <Badge variant={verified ? "default" : "outline"} className="gap-1">
        {verified ? <BadgeCheck /> : <ShieldOff />}
        {verified ? "Verified" : "Unverified"}
      </Badge>
    </Button>
  );
}
