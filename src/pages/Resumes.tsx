import { useEffect, useState } from "react";
import { FileText, HardDriveDownload, ShieldCheck, Sparkles, Upload } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import DeleteButton from "@/components/profile/DeleteButton";
import SectionCard from "@/components/profile/SectionCard";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import ExtractionReview from "@/components/resumes/ExtractionReview";
import {
  ipc,
  type ExtractedProfile,
  type LatexStatus,
  type ResumeFile,
  type ResumeFileKind,
  type ImportCounts,
} from "@/lib/ipc";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function FileList({
  files,
  onDelete,
  onView,
  extraAction,
}: {
  files: ResumeFile[];
  onDelete: (id: number) => Promise<void>;
  onView?: (file: ResumeFile) => void;
  extraAction?: (file: ResumeFile) => React.ReactNode;
}) {
  if (files.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        Nothing uploaded yet — use the upload button above.
      </p>
    );
  }
  return (
    <ul className="divide-y">
      {files.map((file) => (
        <li key={file.id} className="group flex items-center gap-3 py-3 text-sm first:pt-0 last:pb-0">
          <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
          <div className="min-w-0 flex-1">
            <p className="truncate font-medium">{file.originalFilename}</p>
            <p className="truncate text-xs text-muted-foreground">
              {formatSize(file.fileSize)} ·{" "}
              {new Date(file.createdAt).toLocaleDateString()} · sha256 {file.sha256.slice(0, 12)}…
            </p>
          </div>
          {extraAction ? extraAction(file) : null}
          {onView ? (
            <Button variant="ghost" size="xs" onClick={() => onView(file)}>
              View source
            </Button>
          ) : null}
          <DeleteButton
            confirmLabel="Delete"
            cancelLabel="Keep"
            onConfirm={async () => {
              try {
                await onDelete(file.id);
                toast.success("File deleted");
              } catch (e) {
                toast.error(String(e));
              }
            }}
          />
        </li>
      ))}
    </ul>
  );
}

export default function Resumes() {
  const [pdfs, setPdfs] = useState<ResumeFile[]>([]);
  const [templates, setTemplates] = useState<ResumeFile[]>([]);
  const [latex, setLatex] = useState<LatexStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [uploading, setUploading] = useState(false);
  const [viewing, setViewing] = useState<{ file: ResumeFile; content: string } | null>(null);
  const [extractingId, setExtractingId] = useState<number | null>(null);
  const [review, setReview] = useState<ExtractedProfile | null>(null);
  const [pasteOpen, setPasteOpen] = useState(false);
  const [pasteText, setPasteText] = useState("");

  const reload = () => {
    Promise.all([
      ipc.resumeFile.list("pdf_master"),
      ipc.resumeFile.list("tex_template"),
      ipc.latexDetect(),
    ])
      .then(([pdfRows, texRows, latexStatus]) => {
        setPdfs(Array.isArray(pdfRows) ? pdfRows : []);
        setTemplates(Array.isArray(texRows) ? texRows : []);
        setLatex(latexStatus ?? { available: false, engine: null });
      })
      .catch((e) => toast.error(String(e)))
      .finally(() => setLoading(false));
  };

  useEffect(reload, []);

  const upload = async (kind: ResumeFileKind) => {
    const isPdf = kind === "pdf_master";
    const selection = await open({
      multiple: false,
      title: isPdf ? "Upload a master resume PDF" : "Upload a LaTeX template",
      filters: [{ name: isPdf ? "Resume PDF" : "LaTeX template", extensions: [isPdf ? "pdf" : "tex"] }],
    });
    if (typeof selection !== "string") return;
    setUploading(true);
    try {
      await ipc.resumeFile.upload(kind, selection);
      toast.success(isPdf ? "Resume stored (immutable original)" : "Template stored (immutable original)");
      reload();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setUploading(false);
    }
  };

  const extractProfile = async (file: ResumeFile) => {
    setExtractingId(file.id);
    try {
      const extracted = await ipc.resumeFile.extractProfile(file.id);
      setReview(extracted);
    } catch (e) {
      const message = String(e);
      if (message.toLowerCase().includes("no readable text layer")) {
        toast.message("No text layer in this PDF", {
          description: "Paste the resume text instead and the AI will structure it.",
        });
        setPasteOpen(true);
      } else {
        toast.error(message);
      }
    } finally {
      setExtractingId(null);
    }
  };

  const extractFromPaste = async () => {
    setExtractingId(-1);
    try {
      const extracted = await ipc.resumeFile.extractFromText(pasteText);
      setPasteOpen(false);
      setPasteText("");
      setReview(extracted);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setExtractingId(null);
    }
  };

  const viewTemplate = async (file: ResumeFile) => {
    try {
      const content = await ipc.resumeFile.texContent(file.id);
      setViewing({ file, content });
    } catch (e) {
      toast.error(String(e));
    }
  };

  const remove = async (id: number) => {
    await ipc.resumeFile.remove(id);
    reload();
  };

  return (
    <section>
      <header className="mb-8">
        <h1 className="text-2xl font-semibold tracking-tight">Resumes</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Your immutable source files. Uploaded originals are stored content-addressed and never
          modified — generated variants live separately.
        </p>
      </header>

      <div className="space-y-6">
        <SectionCard
          title="Master resumes"
          description="The PDFs you currently send out. These are the baseline for tailored variants later."
          action={
            <Button onClick={() => void upload("pdf_master")} disabled={uploading}>
              <Upload /> {uploading ? "Uploading…" : "Upload PDF"}
            </Button>
          }
        >
          {loading ? (
            <p className="text-sm text-muted-foreground">Loading…</p>
          ) : pdfs.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              Nothing uploaded yet — upload a master resume, then extract its contents into your
              career profile.
            </p>
          ) : (
            <FileList
              files={pdfs}
              onDelete={remove}
              onView={undefined}
              extraAction={(file) => (
                <Button
                  variant="ghost"
                  size="xs"
                  disabled={extractingId !== null}
                  onClick={() => void extractProfile(file)}
                >
                  <Sparkles /> {extractingId === file.id ? "Extracting…" : "Extract profile"}
                </Button>
              )}
            />
          )}
        </SectionCard>

        <SectionCard
          title="LaTeX template"
          description="Your Jake's-style .tex source. Used as the base for generating tailored variants."
          action={
            <Button variant="outline" onClick={() => void upload("tex_template")} disabled={uploading}>
              <Upload /> Upload .tex
            </Button>
          }
        >
          <div className="mb-3 flex items-center gap-2 text-xs">
            <Badge variant={latex?.available ? "default" : "secondary"}>
              {latex?.available
                ? `LaTeX ready · ${latex.engine}`
                : "No LaTeX engine found — .tex export only"}
            </Badge>
            {latex && !latex.available ? (
              <span className="text-muted-foreground">
                Install TeX Live (pdflatex) or tectonic to compile PDFs locally.
              </span>
            ) : null}
          </div>
          {loading ? (
            <p className="text-sm text-muted-foreground">Loading…</p>
          ) : (
            <FileList files={templates} onDelete={remove} onView={(f) => void viewTemplate(f)} />
          )}
        </SectionCard>

        <SectionCard title="How storage works">
          <ul className="space-y-1.5 text-sm text-muted-foreground">
            <li className="flex items-start gap-2">
              <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-emerald-600" />
              Files are copied into the app's data directory, hashed (sha256) and never touched
              again — re-uploading the same file is a no-op.
            </li>
            <li className="flex items-start gap-2">
              <HardDriveDownload className="mt-0.5 h-4 w-4 shrink-0 text-emerald-600" />
              Deleting here removes the stored copy only; your original file on disk is untouched.
            </li>
          </ul>
        </SectionCard>
      </div>

      {pasteOpen ? (
        <Dialog open onOpenChange={(o) => !o && setPasteOpen(false)}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Paste resume text</DialogTitle>
              <DialogDescription>
                This PDF has no readable text layer. Paste the resume content and the AI will
                structure it.
              </DialogDescription>
            </DialogHeader>
            <Textarea
              rows={10}
              value={pasteText}
              placeholder="Paste the full resume text here…"
              onChange={(e) => setPasteText(e.target.value)}
            />
            <DialogFooter>
              <Button variant="outline" onClick={() => setPasteOpen(false)}>
                Cancel
              </Button>
              <Button
                disabled={extractingId === -1 || pasteText.trim().length < 40}
                onClick={() => void extractFromPaste()}
              >
                {extractingId === -1 ? "Structuring…" : "Structure with AI"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      ) : null}

      {review ? (
        <ExtractionReview
          profile={review}
          onClose={() => setReview(null)}
          onImported={(counts: ImportCounts) => {
            const total =
              counts.education + counts.experience + counts.projects + counts.skills +
              counts.certifications + counts.achievements + counts.links +
              (counts.identityUpdated ? 1 : 0);
            toast.success(
              `Imported ${total} item${total === 1 ? "" : "s"} into your career profile (verified)`,
            );
          }}
        />
      ) : null}

      {viewing ? (
        <Dialog open onOpenChange={(o) => !o && setViewing(null)}>
          <DialogContent className="sm:max-w-2xl">
            <DialogHeader>
              <DialogTitle>{viewing.file.originalFilename}</DialogTitle>
              <DialogDescription>LaTeX source (read-only)</DialogDescription>
            </DialogHeader>
            <pre className="max-h-[60vh] overflow-auto whitespace-pre-wrap rounded-md bg-muted p-3 text-xs">
              {viewing.content}
            </pre>
          </DialogContent>
        </Dialog>
      ) : null}
    </section>
  );
}
