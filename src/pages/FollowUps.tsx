import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { BellRing, CalendarClock, Pencil, Wand2 } from "lucide-react";
import { toast } from "sonner";
import DeleteButton from "@/components/profile/DeleteButton";
import EmptyState from "@/components/EmptyState";
import SectionCard from "@/components/profile/SectionCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ipc, type Application, type Contact, type FollowUp, type FollowUpConfig } from "@/lib/ipc";

function formatDay(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso.slice(0, 10) : d.toLocaleDateString();
}

/** Converts a yyyy-mm-dd input value to an RFC 3339 timestamp (local noon). */
function dayToRfc3339(day: string): string {
  const [y, m, d] = day.split("-").map(Number);
  return new Date(Date.UTC(y, (m ?? 1) - 1, d ?? 1, 12)).toISOString();
}

function ConfigCard({ onSaved }: { onSaved: (config: FollowUpConfig) => void }) {
  const [config, setConfig] = useState<FollowUpConfig | null>(null);
  const [days, setDays] = useState("7");
  const [secondDays, setSecondDays] = useState("");
  const [autoSchedule, setAutoSchedule] = useState(true);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    ipc.followUp
      .configGet()
      .then((cfg) => {
        setConfig(cfg);
        setDays(String(cfg.days));
        setSecondDays(cfg.secondDays ? String(cfg.secondDays) : "");
        setAutoSchedule(cfg.autoSchedule);
      })
      .catch((e) => toast.error(String(e)));
  }, []);

  const save = async () => {
    const parsedDays = Number(days);
    const parsedSecond = secondDays.trim() === "" ? null : Number(secondDays);
    if (!Number.isInteger(parsedDays) || parsedDays < 1) {
      toast.error("Follow-up interval must be a whole number of days (≥ 1).");
      return;
    }
    if (parsedSecond !== null && (!Number.isInteger(parsedSecond) || parsedSecond < 1)) {
      toast.error("Second follow-up interval must be a whole number of days (≥ 1) or empty.");
      return;
    }
    setBusy(true);
    try {
      const saved = await ipc.followUp.configSet({
        days: parsedDays,
        secondDays: parsedSecond,
        autoSchedule,
      });
      setConfig(saved);
      onSaved(saved);
      toast.success("Follow-up settings saved");
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SectionCard
      title="Cadence"
      description="Applied automatically to every sent email that is linked to an application."
    >
      {!config ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : (
        <div className="grid gap-4 sm:grid-cols-3">
          <div>
            <Label htmlFor="fu-days" className="mb-1.5">
              Follow up after (days)
            </Label>
            <Input
              id="fu-days"
              type="number"
              min={1}
              value={days}
              onChange={(e) => setDays(e.target.value)}
            />
          </div>
          <div>
            <Label htmlFor="fu-second" className="mb-1.5">
              Second follow-up (days, optional)
            </Label>
            <Input
              id="fu-second"
              type="number"
              min={1}
              placeholder="Disabled"
              value={secondDays}
              onChange={(e) => setSecondDays(e.target.value)}
            />
          </div>
          <div>
            <Label htmlFor="fu-auto" className="mb-1.5">
              Auto-schedule on send
            </Label>
            <Button
              id="fu-auto"
              variant={autoSchedule ? "default" : "outline"}
              className="w-full"
              onClick={() => setAutoSchedule((v) => !v)}
            >
              {autoSchedule ? "Enabled" : "Disabled"}
            </Button>
          </div>
        </div>
      )}
      <div className="mt-4">
        <Button onClick={() => void save()} disabled={busy || !config}>
          Save cadence
        </Button>
      </div>
    </SectionCard>
  );
}

interface FollowUpsProps {
  onDueCountChange?: (count: number) => void;
}

export default function FollowUps({ onDueCountChange }: FollowUpsProps = {}) {
  const navigate = useNavigate();
  const [due, setDue] = useState<FollowUp[]>([]);
  const [upcoming, setUpcoming] = useState<FollowUp[]>([]);
  const [finished, setFinished] = useState<FollowUp[]>([]);
  const [applications, setApplications] = useState<Application[]>([]);
  const [contacts, setContacts] = useState<Contact[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [draftingId, setDraftingId] = useState<number | null>(null);
  const [rescheduleTarget, setRescheduleTarget] = useState<FollowUp | null>(null);
  const [rescheduleDay, setRescheduleDay] = useState("");

  const reload = () => {
    Promise.all([
      ipc.followUp.due(),
      ipc.followUp.list("pending"),
      ipc.followUp.list("sent"),
      ipc.followUp.list("cancelled"),
      ipc.followUp.list("suppressed"),
      ipc.application.list(),
      ipc.contact.list(),
    ])
      .then(([dueRows, pendingRows, sentRows, cancelledRows, suppressedRows, apps, cts]) => {
        setDue(Array.isArray(dueRows) ? dueRows : []);
        setUpcoming(Array.isArray(pendingRows) ? pendingRows : []);
        setFinished(
          [
            ...(Array.isArray(sentRows) ? sentRows : []),
            ...(Array.isArray(cancelledRows) ? cancelledRows : []),
            ...(Array.isArray(suppressedRows) ? suppressedRows : []),
          ].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)),
        );
        setApplications(Array.isArray(apps) ? apps : []);
        setContacts(Array.isArray(cts) ? cts : []);
        const dueCount = Array.isArray(dueRows) ? dueRows.length : 0;
        onDueCountChange?.(dueCount);
      })
      .catch((e) => toast.error(String(e)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  };

  useEffect(reload, []);

  const appLabel = (id: number) => {
    const app = applications.find((a) => a.id === id);
    return app ? `${app.company} · ${app.role}` : `application #${id}`;
  };
  const contactLabel = (id: number | null) => {
    if (id === null) return null;
    const contact = contacts.find((c) => c.id === id);
    return contact ? contact.name || contact.email : null;
  };

  const draft = async (followUp: FollowUp) => {
    setDraftingId(followUp.id);
    try {
      const created = await ipc.followUp.draft(followUp.id);
      toast.success("Follow-up draft ready — review it in Emails");
      navigate("/emails", { state: { draftId: created.id } });
    } catch (e) {
      toast.error(String(e));
    } finally {
      setDraftingId(null);
    }
  };

  const reschedule = async () => {
    if (!rescheduleTarget || !rescheduleDay) return;
    setBusy(true);
    try {
      await ipc.followUp.reschedule(rescheduleTarget.id, dayToRfc3339(rescheduleDay));
      toast.success("Follow-up rescheduled");
      setRescheduleTarget(null);
      setRescheduleDay("");
      reload();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusy(false);
    }
  };

  const renderRow = (row: FollowUp, actions: boolean) => {
    const muted = ["sent", "cancelled", "suppressed"].includes(row.status);
    return (
      <li
        key={row.id}
        className={`flex items-center gap-3 py-2.5 text-sm first:pt-0 last:pb-0 ${muted ? "opacity-60" : ""}`}
      >
        <div className="min-w-0 flex-1">
          <p className="flex flex-wrap items-center gap-2 font-medium">
            {appLabel(row.applicationId)}
            <Badge variant={row.status === "due" ? "default" : "outline"}>
              {row.status === "due" ? `Due · #${row.sequence}` : `#${row.sequence}`}
            </Badge>
          </p>
          <p className="text-xs text-muted-foreground">
            {[
              contactLabel(row.contactId) ?? "no contact linked",
              muted && row.suppressedReason ? row.suppressedReason : null,
            ]
              .filter(Boolean)
              .join(" · ")}
          </p>
        </div>
        <span className="shrink-0 text-xs text-muted-foreground">
          {formatDay(row.scheduledFor)}
        </span>
        {actions ? (
          <div className="flex shrink-0 items-center gap-1">
            <Button
              variant="outline"
              size="xs"
              disabled={draftingId === row.id}
              onClick={() => void draft(row)}
            >
              <Wand2 /> {draftingId === row.id ? "Drafting…" : "Draft follow-up"}
            </Button>
            <Button
              variant="ghost"
              size="icon-xs"
              aria-label="Reschedule"
              onClick={() => {
                setRescheduleTarget(row);
                setRescheduleDay(row.scheduledFor.slice(0, 10));
              }}
            >
              <CalendarClock />
            </Button>
            <DeleteButton
              confirmLabel="Cancel"
              cancelLabel="Keep"
              onConfirm={async () => {
                try {
                  await ipc.followUp.cancel(row.id);
                  toast.success("Follow-up cancelled");
                  reload();
                } catch (e) {
                  toast.error(String(e));
                }
              }}
            />
          </div>
        ) : (
          <Badge variant="secondary">{row.status}</Badge>
        )}
      </li>
    );
  };

  return (
    <section>
      <header className="mb-8 flex items-center gap-3">
        <BellRing className="h-6 w-6 text-primary" />
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Follow-ups</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Scheduled automatically after outreach. Suppressed the moment a contact replies or
            an application closes.
          </p>
        </div>
      </header>

      <div className="space-y-6">
        <ConfigCard onSaved={reload} />

        <SectionCard title={`Due${due.length ? ` (${due.length})` : ""}`}>
          {loading ? (
            <p className="text-sm text-muted-foreground">Loading…</p>
          ) : due.length === 0 ? (
            <EmptyState
              icon={BellRing}
              title="Nothing due right now"
              description="Follow-ups appear here once their scheduled date arrives."
            />
          ) : (
            <ul className="divide-y">{due.map((row) => renderRow(row, true))}</ul>
          )}
        </SectionCard>

        <SectionCard title={`Upcoming${upcoming.length ? ` (${upcoming.length})` : ""}`}>
          {loading ? (
            <p className="text-sm text-muted-foreground">Loading…</p>
          ) : upcoming.length === 0 ? (
            <p className="text-sm text-muted-foreground">Nothing scheduled ahead.</p>
          ) : (
            <ul className="divide-y">{upcoming.map((row) => renderRow(row, true))}</ul>
          )}
        </SectionCard>

        {finished.length > 0 ? (
          <SectionCard title={`Completed & suppressed (${finished.length})`}>
            <ul className="max-h-80 divide-y overflow-y-auto pr-1">
              {finished.map((row) => renderRow(row, false))}
            </ul>
          </SectionCard>
        ) : null}
      </div>

      {rescheduleTarget ? (
        <Dialog open onOpenChange={(o) => !o && setRescheduleTarget(null)}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Reschedule follow-up</DialogTitle>
              <DialogDescription>{appLabel(rescheduleTarget.applicationId)}</DialogDescription>
            </DialogHeader>
            <div className="space-y-2">
              <Label htmlFor="reschedule-day" className="mb-1.5">
                New date
              </Label>
              <Input
                id="reschedule-day"
                type="date"
                value={rescheduleDay}
                onChange={(e) => setRescheduleDay(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && rescheduleDay) void reschedule();
                }}
              />
              <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
                <Pencil className="h-3 w-3" /> Pick a date at least one day out.
              </p>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setRescheduleTarget(null)} disabled={busy}>
                Cancel
              </Button>
              <Button onClick={() => void reschedule()} disabled={busy || !rescheduleDay}>
                {busy ? "Saving…" : "Reschedule"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      ) : null}
    </section>
  );
}
