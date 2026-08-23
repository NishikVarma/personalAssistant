import { useEffect, useState } from "react";
import { Pencil, Plus } from "lucide-react";
import { toast } from "sonner";
import DeleteButton from "@/components/profile/DeleteButton";
import EntityAttachments from "@/components/profile/EntityAttachments";
import EmptyState from "@/components/EmptyState";
import FormDialog, { emptyToNull, type FormValues } from "@/components/profile/FormDialog";
import { PROJECT_STATUS_OPTIONS, labelFor, shortDate } from "@/components/profile/labels";
import SectionCard from "@/components/profile/SectionCard";
import VerifiedToggle from "@/components/profile/VerifiedToggle";
import { Badge } from "@/components/ui/badge";
import { FolderGit2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ipc, type Project, type ProjectInput, type ProjectStatus } from "@/lib/ipc";

const FIELDS = [
  { name: "name", label: "Project name", type: "text" as const, required: true },
  {
    name: "status",
    label: "Status",
    type: "select" as const,
    options: PROJECT_STATUS_OPTIONS,
  },
  { name: "repoUrl", label: "Repository URL", type: "text" as const, placeholder: "https://github.com/…" },
  { name: "liveUrl", label: "Live URL", type: "text" as const, placeholder: "https://…" },
  { name: "startedOn", label: "Started on", type: "date" as const },
  { name: "endedOn", label: "Ended on", type: "date" as const },
  {
    name: "description",
    label: "Description",
    type: "textarea" as const,
    placeholder: "What it does and why it matters. Add verified resume bullets below after saving.",
  },
];

function toInitial(item: Project): FormValues {
  return {
    name: item.name,
    status: item.status,
    repoUrl: item.repoUrl ?? "",
    liveUrl: item.liveUrl ?? "",
    startedOn: item.startedOn ?? "",
    endedOn: item.endedOn ?? "",
    description: item.description,
  };
}

function toInput(values: FormValues): ProjectInput {
  return {
    name: values.name.trim(),
    status: values.status as ProjectStatus,
    repoUrl: emptyToNull(values.repoUrl),
    liveUrl: emptyToNull(values.liveUrl),
    startedOn: emptyToNull(values.startedOn),
    endedOn: emptyToNull(values.endedOn),
    description: values.description.trim(),
  };
}

export default function ProjectsSection() {
  const [items, setItems] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [dialog, setDialog] = useState<{ mode: "add" } | { mode: "edit"; item: Project } | null>(
    null,
  );

  const reload = () => {
    ipc.project
      .list()
      .then(setItems)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  };

  useEffect(reload, []);

  return (
    <SectionCard
      title="Projects"
      description="Add each project once; the system reuses it across resumes and applications."
      action={
        <Button variant="outline" size="sm" onClick={() => setDialog({ mode: "add" })}>
          <Plus /> Add
        </Button>
      }
    >
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {loading ? (
        <div className="space-y-2">
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-5/6" />
        </div>
      ) : items.length === 0 ? (
        <EmptyState
          icon={FolderGit2}
          title="No projects added yet"
          description="Add each project once; the system reuses it across resumes and applications."
        />
      ) : (
        <ul className="divide-y">
          {items.map((item) => (
            <li key={item.id} className="py-4 first:pt-0 last:pb-0">
              <div className="flex items-start gap-3">
                <div className="min-w-0 flex-1">
                  <p className="flex flex-wrap items-center gap-2 text-sm font-medium">
                    {item.name}
                    <Badge variant="outline">{labelFor(PROJECT_STATUS_OPTIONS, item.status)}</Badge>
                  </p>
                  <p className="text-sm text-muted-foreground">
                    {[
                      shortDate(item.startedOn) || item.endedOn
                        ? [shortDate(item.startedOn), shortDate(item.endedOn)]
                            .filter(Boolean)
                            .join(" – ")
                        : "",
                      item.repoUrl,
                      item.liveUrl,
                    ]
                      .filter(Boolean)
                      .join(" · ")}
                  </p>
                  {item.description ? <p className="mt-1 text-sm">{item.description}</p> : null}
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  <VerifiedToggle
                    verified={item.verified}
                    onToggle={async () => {
                      try {
                        await ipc.project.setVerified(item.id, !item.verified);
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
                        await ipc.project.remove(item.id);
                        toast.success("Project deleted (bullets and skill links cleaned up)");
                        reload();
                      } catch (e) {
                        toast.error(String(e));
                      }
                    }}
                  />
                </div>
              </div>
              <EntityAttachments entityType="project" entityId={item.id} />
            </li>
          ))}
        </ul>
      )}

      {dialog ? (
        <FormDialog
          title={dialog.mode === "add" ? "Add project" : "Edit project"}
          fields={FIELDS}
          initial={dialog.mode === "edit" ? toInitial(dialog.item) : { status: "completed" }}
          onSubmit={async (values) => {
            if (dialog.mode === "add") {
              await ipc.project.create(toInput(values));
              toast.success("Project added");
            } else {
              await ipc.project.update(dialog.item.id, toInput(values));
              toast.success("Project updated");
            }
            reload();
          }}
          onClose={() => setDialog(null)}
        />
      ) : null}
    </SectionCard>
  );
}
