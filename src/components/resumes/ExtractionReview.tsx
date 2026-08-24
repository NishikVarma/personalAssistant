import { useState } from "react";
import { Check, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  ipc,
  type ExtractedProfile,
  type ImportCounts,
} from "@/lib/ipc";

type Selection = Record<string, boolean>;

interface ExtractionReviewProps {
  profile: ExtractedProfile;
  onClose: () => void;
  onImported: (counts: ImportCounts) => void;
}

function ItemHeader({
  selected,
  onToggle,
  title,
}: {
  selected: boolean;
  onToggle: () => void;
  title: string;
}) {
  return (
    <div className="flex items-center gap-2">
      <input
        type="checkbox"
        checked={selected}
        onChange={onToggle}
        aria-label={`Include ${title}`}
      />
      <span className={selected ? "text-sm font-medium" : "text-sm font-medium opacity-50"}>
        {title}
      </span>
    </div>
  );
}

export default function ExtractionReview({ profile, onClose, onImported }: ExtractionReviewProps) {
  const [draft, setDraft] = useState<ExtractedProfile>(() => JSON.parse(JSON.stringify(profile)));
  const [selected, setSelected] = useState<Selection>(() => {
    const initial: Selection = { identity: true };
    profile.education.forEach((_, i) => (initial[`education:${i}`] = true));
    profile.experience.forEach((_, i) => (initial[`experience:${i}`] = true));
    profile.projects.forEach((_, i) => (initial[`projects:${i}`] = true));
    profile.skills.forEach((_, i) => (initial[`skills:${i}`] = true));
    profile.certifications.forEach((_, i) => (initial[`certifications:${i}`] = true));
    profile.achievements.forEach((_, i) => (initial[`achievements:${i}`] = true));
    profile.links.forEach((_, i) => (initial[`links:${i}`] = true));
    return initial;
  });
  const [busy, setBusy] = useState(false);

  const toggle = (key: string) =>
    setSelected((prev) => ({ ...prev, [key]: !prev[key] }));

  const patch = <K extends keyof ExtractedProfile>(section: K, index: number, field: string, value: string) => {
    setDraft((prev) => {
      const items = [...(prev[section] as unknown as Record<string, unknown>[])];
      items[index] = { ...(items[index] as Record<string, unknown>), [field]: value };
      return { ...prev, [section]: items };
    });
  };

  const removeItem = <K extends keyof ExtractedProfile>(section: K, index: number) => {
    setDraft((prev) => {
      const items = [...(prev[section] as unknown as Record<string, unknown>[])];
      items.splice(index, 1);
      return { ...prev, [section]: items };
    });
  };

  const importSelected = async () => {
    setBusy(true);
    try {
      const payload: ExtractedProfile = {
        ...draft,
        education: draft.education.filter((_, i) => selected[`education:${i}`]),
        experience: draft.experience.filter((_, i) => selected[`experience:${i}`]),
        projects: draft.projects.filter((_, i) => selected[`projects:${i}`]),
        skills: draft.skills.filter((_, i) => selected[`skills:${i}`]),
        certifications: draft.certifications.filter((_, i) => selected[`certifications:${i}`]),
        achievements: draft.achievements.filter((_, i) => selected[`achievements:${i}`]),
        links: draft.links.filter((_, i) => selected[`links:${i}`]),
      };
      const counts = await ipc.resumeFile.importExtracted(payload, true);
      onImported(counts);
      onClose();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusy(false);
    }
  };

  const totalSelected =
    (selected.identity ? 1 : 0) +
    Object.entries(selected).filter(([key, value]) => value && key.includes(":")).length;

  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Review extracted profile</DialogTitle>
          <DialogDescription>
            Correct anything the AI got wrong, untick what you do not want, then import.
            Imported items are marked verified.
          </DialogDescription>
        </DialogHeader>

        <div className="max-h-[55vh] space-y-5 overflow-y-auto pr-1">
          <div>
            <ItemHeader
              selected={Boolean(selected.identity)}
              onToggle={() => toggle("identity")}
              title="Identity"
            />
            {selected.identity ? (
              <div className="mt-2 grid gap-2 sm:grid-cols-2">
                {(
                  [
                    ["fullName", "Full name"],
                    ["email", "Email"],
                    ["phone", "Phone"],
                    ["location", "Location"],
                  ] as const
                ).map(([field, label]) => (
                  <div key={field}>
                    <Label className="mb-1 text-xs">{label}</Label>
                    <Input
                      className="h-7"
                      value={draft[field]}
                      onChange={(e) => setDraft((prev) => ({ ...prev, [field]: e.target.value }))}
                    />
                  </div>
                ))}
                <div className="sm:col-span-2">
                  <Label className="mb-1 text-xs">Summary</Label>
                  <Textarea
                    className="min-h-12"
                    value={draft.summary}
                    onChange={(e) => setDraft((prev) => ({ ...prev, summary: e.target.value }))}
                  />
                </div>
              </div>
            ) : null}
          </div>

          {draft.education.length > 0 ? (
            <div>
              <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Education
              </p>
              <div className="space-y-2">
                {draft.education.map((item, index) => (
                  <div key={index} className="rounded-lg border border-border p-2.5">
                    <div className="flex items-center justify-between">
                      <ItemHeader
                        selected={Boolean(selected[`education:${index}`])}
                        onToggle={() => toggle(`education:${index}`)}
                        title={item.institution || "(no institution)"}
                      />
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        aria-label={`Remove education ${index + 1}`}
                        onClick={() => removeItem("education", index)}
                      >
                        <Trash2 />
                      </Button>
                    </div>
                    {selected[`education:${index}`] ? (
                      <div className="mt-2 grid gap-2 sm:grid-cols-2">
                        <Input
                          className="h-7"
                          value={item.institution}
                          placeholder="Institution"
                          onChange={(e) => patch("education", index, "institution", e.target.value)}
                        />
                        <Input
                          className="h-7"
                          value={item.degree}
                          placeholder="Degree"
                          onChange={(e) => patch("education", index, "degree", e.target.value)}
                        />
                      </div>
                    ) : null}
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          {draft.experience.length > 0 ? (
            <div>
              <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Experience
              </p>
              <div className="space-y-2">
                {draft.experience.map((item, index) => (
                  <div key={index} className="rounded-lg border border-border p-2.5">
                    <div className="flex items-center justify-between">
                      <ItemHeader
                        selected={Boolean(selected[`experience:${index}`])}
                        onToggle={() => toggle(`experience:${index}`)}
                        title={item.organization || "(no organization)"}
                      />
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        aria-label={`Remove experience ${index + 1}`}
                        onClick={() => removeItem("experience", index)}
                      >
                        <Trash2 />
                      </Button>
                    </div>
                    {selected[`experience:${index}`] ? (
                      <div className="mt-2 grid gap-2 sm:grid-cols-2">
                        <Input
                          className="h-7"
                          value={item.organization}
                          placeholder="Organization"
                          onChange={(e) => patch("experience", index, "organization", e.target.value)}
                        />
                        <Input
                          className="h-7"
                          value={item.title}
                          placeholder="Title"
                          onChange={(e) => patch("experience", index, "title", e.target.value)}
                        />
                      </div>
                    ) : null}
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          {draft.projects.length > 0 ? (
            <div>
              <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Projects
              </p>
              <div className="space-y-2">
                {draft.projects.map((item, index) => (
                  <div key={index} className="rounded-lg border border-border p-2.5">
                    <div className="flex items-center justify-between">
                      <ItemHeader
                        selected={Boolean(selected[`projects:${index}`])}
                        onToggle={() => toggle(`projects:${index}`)}
                        title={item.name || "(no name)"}
                      />
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        aria-label={`Remove project ${index + 1}`}
                        onClick={() => removeItem("projects", index)}
                      >
                        <Trash2 />
                      </Button>
                    </div>
                    {selected[`projects:${index}`] ? (
                      <Input
                        className="mt-2 h-7"
                        value={item.name}
                        placeholder="Project name"
                        onChange={(e) => patch("projects", index, "name", e.target.value)}
                      />
                    ) : null}
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          {draft.skills.length > 0 ? (
            <div>
              <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Skills
              </p>
              <div className="space-y-1.5">
                {draft.skills.map((skill, index) => (
                  <div key={index} className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={Boolean(selected[`skills:${index}`])}
                      onChange={() => toggle(`skills:${index}`)}
                      aria-label={`Include ${skill.name}`}
                    />
                    <Input
                      className="h-7 max-w-xs"
                      value={skill.name}
                      onChange={(e) => patch("skills", index, "name", e.target.value)}
                    />
                    <span className="text-xs text-muted-foreground">{skill.category ?? ""}</span>
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          {draft.certifications.length > 0 ? (
            <div>
              <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Certifications
              </p>
              <div className="space-y-1.5">
                {draft.certifications.map((cert, index) => (
                  <div key={index} className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={Boolean(selected[`certifications:${index}`])}
                      onChange={() => toggle(`certifications:${index}`)}
                      aria-label={`Include ${cert.name}`}
                    />
                    <Input
                      className="h-7 max-w-sm"
                      value={cert.name}
                      onChange={(e) => patch("certifications", index, "name", e.target.value)}
                    />
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          {draft.achievements.length > 0 ? (
            <div>
              <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Achievements
              </p>
              <div className="space-y-1.5">
                {draft.achievements.map((achievement, index) => (
                  <div key={index} className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={Boolean(selected[`achievements:${index}`])}
                      onChange={() => toggle(`achievements:${index}`)}
                      aria-label={`Include ${achievement.title}`}
                    />
                    <Input
                      className="h-7 max-w-sm"
                      value={achievement.title}
                      onChange={(e) => patch("achievements", index, "title", e.target.value)}
                    />
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          {draft.links.length > 0 ? (
            <div>
              <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Links
              </p>
              <div className="space-y-1.5">
                {draft.links.map((link, index) => (
                  <div key={index} className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={Boolean(selected[`links:${index}`])}
                      onChange={() => toggle(`links:${index}`)}
                      aria-label={`Include ${link.url}`}
                    />
                    <Input
                      className="h-7 flex-1"
                      value={link.url}
                      onChange={(e) => patch("links", index, "url", e.target.value)}
                    />
                    <span className="text-xs text-muted-foreground">{link.kind ?? ""}</span>
                  </div>
                ))}
              </div>
            </div>
          ) : null}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={busy}>
            Cancel
          </Button>
          <Button disabled={busy || totalSelected === 0} onClick={() => void importSelected()}>
            <Check /> {busy ? "Importing…" : `Import ${totalSelected} item${totalSelected === 1 ? "" : "s"}`}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
