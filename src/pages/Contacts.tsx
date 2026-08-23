import { useEffect, useState } from "react";
import { Pencil, Plus, X } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import ContactTagsEditor from "@/components/contacts/ContactTagsEditor";
import DeleteButton from "@/components/profile/DeleteButton";
import FormDialog, {
  emptyToNull,
  type FormValues,
} from "@/components/profile/FormDialog";
import SectionCard from "@/components/profile/SectionCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ipc, type Contact, type ContactInput } from "@/lib/ipc";

const FIELDS = [
  { name: "name", label: "Name", type: "text" as const },
  { name: "email", label: "Email", type: "text" as const, required: true },
  { name: "organization", label: "Organization", type: "text" as const },
  { name: "roleTitle", label: "Role / title", type: "text" as const },
  {
    name: "linkedinUrl",
    label: "LinkedIn URL",
    type: "text" as const,
    full: true,
    placeholder: "https://linkedin.com/in/…",
  },
  {
    name: "notes",
    label: "Notes",
    type: "textarea" as const,
    placeholder: "Context about this contact…",
  },
];

function toInitial(contact: Contact): FormValues {
  return {
    name: contact.name,
    email: contact.email,
    organization: contact.organization ?? "",
    roleTitle: contact.roleTitle ?? "",
    linkedinUrl: contact.linkedinUrl ?? "",
    notes: contact.notes,
  };
}

function toInput(values: FormValues): ContactInput {
  return {
    name: values.name.trim(),
    email: values.email.trim(),
    organization: emptyToNull(values.organization),
    roleTitle: emptyToNull(values.roleTitle),
    linkedinUrl: emptyToNull(values.linkedinUrl),
    notes: values.notes.trim(),
  };
}

function formatLastContacted(value: string | null): string {
  if (!value) return "";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleDateString();
}

export default function Contacts() {
  const [contacts, setContacts] = useState<Contact[]>([]);
  const [tags, setTags] = useState<Array<{ id: number; name: string; color: string | null }>>([]);
  const [search, setSearch] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [dialog, setDialog] = useState<{ mode: "add" } | { mode: "edit"; item: Contact } | null>(
    null,
  );
  const [newTag, setNewTag] = useState("");

  const reloadContacts = (term = search) => {
    ipc.contact
      .list(term)
      .then(setContacts)
      .catch((e) => setError(String(e)));
  };

  const reloadTags = () => ipc.tag.list().then(setTags).catch(() => setTags([]));

  useEffect(() => {
    reloadContacts();
    reloadTags();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const runSearch = () => reloadContacts();

  const addTag = async () => {
    if (!newTag.trim()) return;
    try {
      await ipc.tag.create({ name: newTag.trim(), color: null });
      setNewTag("");
      reloadTags();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <section>
      <header className="mb-8 flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Contacts</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Recruiters and referrals with their history. Duplicate or recent outreach is flagged
            before anything is sent.
          </p>
        </div>
        <Button onClick={() => setDialog({ mode: "add" })}>
          <Plus /> Add contact
        </Button>
      </header>

      <div className="space-y-6">
        <SectionCard title="Tags" description="Reusable labels for grouping contacts.">
          <div className="flex flex-wrap items-center gap-1.5">
            {tags.map((tag) => (
              <Badge key={tag.id} variant="outline" className="gap-0.5 pr-1">
                {tag.name}
                <button
                  type="button"
                  aria-label={`Delete ${tag.name}`}
                  className="rounded-full p-0.5 hover:bg-foreground/10"
                  onClick={async () => {
                    await ipc.tag.remove(tag.id);
                    reloadTags();
                    reloadContacts();
                  }}
                >
                  <X className="size-3" />
                </button>
              </Badge>
            ))}
            {tags.length === 0 ? (
              <span className="text-sm text-muted-foreground">No tags yet.</span>
            ) : null}
          </div>
          <div className="mt-3 flex items-center gap-2">
            <Input
              className="h-7 w-44"
              placeholder="New tag…"
              value={newTag}
              onChange={(e) => setNewTag(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void addTag();
              }}
            />
            <Button variant="outline" size="xs" disabled={!newTag.trim()} onClick={addTag}>
              <Plus /> Add
            </Button>
          </div>
        </SectionCard>

        <SectionCard
          title={`All contacts${contacts.length ? ` (${contacts.length})` : ""}`}
          action={
            <div className="flex items-center gap-2">
              <Input
                className="h-8 w-56"
                placeholder="Search name, email, org…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") runSearch();
                }}
              />
              <Button variant="outline" size="sm" onClick={runSearch}>
                Search
              </Button>
            </div>
          }
        >
          {error ? <p className="mb-2 text-sm text-destructive">{error}</p> : null}
          {contacts.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              {search ? "No contacts match this search." : "No contacts added yet."}
            </p>
          ) : (
            <ul className="divide-y">
              {contacts.map((contact) => (
                <li key={contact.id} className="py-4 first:pt-0 last:pb-0">
                  <div className="flex items-start gap-3">
                    <div className="min-w-0 flex-1">
                      <p className="flex flex-wrap items-baseline gap-2 text-sm font-medium">
                        {contact.name || "(no name)"}
                        <a
                          href={`mailto:${contact.email}`}
                          className="text-sm font-normal text-muted-foreground underline-offset-2 hover:underline"
                          onClick={(e) => e.preventDefault()}
                        >
                          {contact.email}
                        </a>
                      </p>
                      <p className="text-sm text-muted-foreground">
                        {[
                          [contact.roleTitle, contact.organization]
                            .filter(Boolean)
                            .join(" at "),
                          contact.lastContactedAt
                            ? `last contacted ${formatLastContacted(contact.lastContactedAt)}`
                            : "never contacted",
                        ]
                          .filter(Boolean)
                          .join(" · ")}
                      </p>
                      {contact.notes ? (
                        <p className="mt-0.5 truncate text-xs text-muted-foreground">
                          {contact.notes}
                        </p>
                      ) : null}
                    </div>
                    <div className="flex shrink-0 items-center gap-1">
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        aria-label="Edit"
                        onClick={() => setDialog({ mode: "edit", item: contact })}
                      >
                        <Pencil />
                      </Button>
                      <DeleteButton
                        onConfirm={async () => {
                          await ipc.contact.remove(contact.id);
                          reloadContacts();
                        }}
                      />
                    </div>
                  </div>
                  {contact.linkedinUrl ? (
                    <button
                      type="button"
                      className="mt-1 block truncate text-xs text-primary underline-offset-2 hover:underline"
                      onClick={() => void openUrl(contact.linkedinUrl!)}
                    >
                      {contact.linkedinUrl}
                    </button>
                  ) : null}
                  <div className="mt-2">
                    <ContactTagsEditor contactId={contact.id} />
                  </div>
                </li>
              ))}
            </ul>
          )}
        </SectionCard>
      </div>

      {dialog ? (
        <FormDialog
          title={dialog.mode === "add" ? "Add contact" : "Edit contact"}
          fields={FIELDS}
          initial={dialog.mode === "edit" ? toInitial(dialog.item) : {}}
          onSubmit={async (values) => {
            if (dialog.mode === "add") {
              await ipc.contact.create(toInput(values));
            } else {
              await ipc.contact.update(dialog.item.id, toInput(values));
            }
            reloadContacts();
          }}
          onClose={() => setDialog(null)}
        />
      ) : null}
    </section>
  );
}
