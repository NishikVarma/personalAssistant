import BulletsEditor from "@/components/profile/BulletsEditor";
import EntitySkillsEditor from "@/components/profile/EntitySkillsEditor";
import type { ProfileEntityType } from "@/lib/ipc";

interface EntityAttachmentsProps {
  entityType: ProfileEntityType;
  entityId: number;
}

/** Nested resume bullets + skill chips for a project or experience entry. */
export default function EntityAttachments({ entityType, entityId }: EntityAttachmentsProps) {
  return (
    <div className="mt-3 space-y-3 rounded-lg border border-dashed border-border p-3">
      <div>
        <p className="mb-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">
          Resume bullets
        </p>
        <BulletsEditor entityType={entityType} entityId={entityId} />
      </div>
      <div>
        <p className="mb-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">
          Skills
        </p>
        <EntitySkillsEditor entityType={entityType} entityId={entityId} />
      </div>
    </div>
  );
}
