import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  ArrowRight,
  BellRing,
  Briefcase,
  CheckCircle2,
  FileText,
  Mail,
  Users,
  XCircle,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { ipc, type AppInfo, type ApplicationStatus, type GeneratedEmail } from "@/lib/ipc";
import { notifyDueFollowUps } from "@/lib/notifications";

const STATUS_LABELS: Record<ApplicationStatus, string> = {
  saved: "Saved",
  preparing: "Preparing",
  applied: "Applied",
  contacted: "Contacted",
  follow_up_due: "Follow-up due",
  response_received: "Response received",
  oa: "OA",
  interview: "Interview",
  offer: "Offer",
  rejected: "Rejected",
  withdrawn: "Withdrawn",
};

const STAGES: { label: string; statuses: ApplicationStatus[]; bar: string }[] = [
  {
    label: "Backlog",
    statuses: ["saved", "preparing"],
    bar: "bg-sky-500/70",
  },
  {
    label: "Outreach",
    statuses: ["applied", "contacted", "follow_up_due"],
    bar: "bg-emerald-500/70",
  },
  {
    label: "Interviews",
    statuses: ["response_received", "oa", "interview"],
    bar: "bg-violet-500/70",
  },
  {
    label: "Closed",
    statuses: ["offer", "rejected", "withdrawn"],
    bar: "bg-zinc-400/60",
  },
];

function formatWhen(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString();
}

interface StatCardProps {
  icon: typeof Users;
  label: string;
  value: number | null;
  hint: string;
  to: string;
}

function StatCard({ icon: Icon, label, value, hint, to }: StatCardProps) {
  return (
    <Link to={to} className="group" title={hint}>
      <Card className="transition-colors group-hover:border-ring">
        <CardContent className="flex items-center gap-4">
          <div className="rounded-lg bg-accent p-2.5 text-accent-foreground">
            <Icon className="h-5 w-5" />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
              {label}
            </p>
            {value === null ? (
              <Skeleton className="mt-1 h-6 w-10" />
            ) : (
              <p className="text-xl leading-tight font-semibold">
                {value}
                <span className="ml-1.5 text-[10px] font-normal text-muted-foreground">
                  {hint}
                </span>
              </p>
            )}
          </div>
          <ArrowRight className="h-4 w-4 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
        </CardContent>
      </Card>
    </Link>
  );
}

export default function Dashboard() {
  const [apps, setApps] = useState<{ count: number; byStatus: Partial<Record<ApplicationStatus, number>>; recent: { id: number; company: string; role: string; status: ApplicationStatus; updatedAt: string }[] } | null>(null);
  const [contactCount, setContactCount] = useState<number | null>(null);
  const [emails, setEmails] = useState<GeneratedEmail[] | null>(null);
  const [dueCount, setDueCount] = useState<number | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    ipc.appInfo().then(setInfo).catch((e) => setError(String(e)));
    ipc.application
      .list()
      .then((list) => {
        const byStatus: Partial<Record<ApplicationStatus, number>> = {};
        for (const app of list) {
          byStatus[app.status] = (byStatus[app.status] ?? 0) + 1;
        }
        setApps({
          count: list.length,
          byStatus,
          recent: list.slice(0, 5).map(({ id, company, role, status, updatedAt }) => ({
            id,
            company,
            role,
            status,
            updatedAt,
          })),
        });
      })
      .catch((e) => setError(String(e)));
    ipc.contact
      .list()
      .then((list) => setContactCount(list.length))
      .catch((e) => setError(String(e)));
    ipc.generatedEmail
      .list()
      .then(setEmails)
      .catch((e) => setError(String(e)));
    ipc.followUp
      .dueCount()
      .then((count) => {
        setDueCount(count);
        void notifyDueFollowUps(count);
      })
      .catch(() => setDueCount(null));
  }, []);

  const draftsInProgress =
    emails?.filter((e) => e.status === "draft" || e.status === "edited").length ?? null;
  const approvedReady = emails?.filter((e) => e.status === "approved").length ?? null;
  const totalForFunnel = apps?.count ?? 0;

  return (
    <section>
      <header className="mb-8">
        <h1 className="text-2xl font-semibold tracking-tight">Dashboard</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Your job search at a glance. Everything below runs locally on your machine.
        </p>
      </header>

      {error ? (
        <Card className="mb-6 border-destructive">
          <CardContent className="pt-6 text-sm text-destructive">
            Backend error: {error}
          </CardContent>
        </Card>
      ) : null}

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
        <StatCard icon={Briefcase} label="Applications" value={apps?.count ?? null} hint="Total tracked" to="/applications" />
        <StatCard icon={Users} label="Contacts" value={contactCount} hint="Recruiters & referrals" to="/contacts" />
        <StatCard icon={FileText} label="Drafts in progress" value={draftsInProgress} hint="Not yet approved" to="/emails" />
        <StatCard icon={Mail} label="Approved ready" value={approvedReady} hint="Waiting to send" to="/emails" />
        <StatCard icon={BellRing} label="Follow-ups due" value={dueCount} hint="Need action" to="/follow-ups" />
      </div>

      <div className="mt-6 grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Pipeline</CardTitle>
            <CardDescription>Applications grouped by stage.</CardDescription>
          </CardHeader>
          <CardContent>
            {!apps ? (
              <div className="space-y-3">
                <Skeleton className="h-7 w-full" />
                <Skeleton className="h-7 w-full" />
                <Skeleton className="h-7 w-full" />
                <Skeleton className="h-7 w-full" />
              </div>
            ) : totalForFunnel === 0 ? (
              <p className="text-sm text-muted-foreground">
                No applications yet — add one to see your pipeline.
              </p>
            ) : (
              <div className="space-y-3">
                {STAGES.map((stage) => {
                  const count = stage.statuses.reduce(
                    (sum, s) => sum + (apps.byStatus[s] ?? 0),
                    0,
                  );
                  const pct = Math.round((count / totalForFunnel) * 100);
                  return (
                    <div key={stage.label}>
                      <div className="mb-1 flex items-center justify-between text-xs">
                        <span className="font-medium">{stage.label}</span>
                        <span className="text-muted-foreground">{count}</span>
                      </div>
                      <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
                        <div
                          className={`h-full rounded-full transition-all ${stage.bar}`}
                          style={{ width: `${Math.max(pct, count > 0 ? 4 : 0)}%` }}
                        />
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Quick actions</CardTitle>
            <CardDescription>Jump straight into a workflow.</CardDescription>
          </CardHeader>
          <CardContent className="grid gap-2">
            <Link
              to="/emails"
              className="flex items-center gap-3 rounded-lg border border-border p-3 text-sm transition-colors hover:bg-accent"
            >
              <Mail className="h-4 w-4 text-primary" />
              Compose an outreach email
              <ArrowRight className="ml-auto h-4 w-4 text-muted-foreground" />
            </Link>
            <Link
              to="/applications"
              className="flex items-center gap-3 rounded-lg border border-border p-3 text-sm transition-colors hover:bg-accent"
            >
              <Briefcase className="h-4 w-4 text-primary" />
              Track a new application
              <ArrowRight className="ml-auto h-4 w-4 text-muted-foreground" />
            </Link>
            <Link
              to="/career-profile"
              className="flex items-center gap-3 rounded-lg border border-border p-3 text-sm transition-colors hover:bg-accent"
            >
              <CheckCircle2 className="h-4 w-4 text-primary" />
              Verify your career profile
              <ArrowRight className="ml-auto h-4 w-4 text-muted-foreground" />
            </Link>
          </CardContent>
        </Card>
      </div>

      <div className="mt-6 grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader className="flex-row items-center justify-between space-y-0">
            <div>
              <CardTitle>Recent applications</CardTitle>
              <CardDescription>Last five you touched.</CardDescription>
            </div>
            <Link
              to="/applications"
              className="text-xs font-medium text-primary hover:underline"
            >
              View all
            </Link>
          </CardHeader>
          <CardContent>
            {!apps ? (
              <div className="space-y-2">
                <Skeleton className="h-8 w-full" />
                <Skeleton className="h-8 w-5/6" />
              </div>
            ) : apps.recent.length === 0 ? (
              <p className="text-sm text-muted-foreground">Nothing here yet.</p>
            ) : (
              <ul className="max-h-56 divide-y overflow-y-auto pr-1">
                {apps.recent.map((app) => (
                  <li key={app.id} className="flex items-center gap-2 py-2 text-sm first:pt-0 last:pb-0">
                    <span className="min-w-0 flex-1 truncate">
                      {app.company} · <span className="text-muted-foreground">{app.role}</span>
                    </span>
                    <Badge variant="outline">{STATUS_LABELS[app.status]}</Badge>
                    <span className="shrink-0 text-xs text-muted-foreground">
                      {formatWhen(app.updatedAt)}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex-row items-center justify-between space-y-0">
            <div>
              <CardTitle>Recent drafts</CardTitle>
              <CardDescription>Your latest AI-generated emails.</CardDescription>
            </div>
            <Link to="/emails" className="text-xs font-medium text-primary hover:underline">
              View all
            </Link>
          </CardHeader>
          <CardContent>
            {!emails ? (
              <div className="space-y-2">
                <Skeleton className="h-8 w-full" />
                <Skeleton className="h-8 w-5/6" />
              </div>
            ) : emails.length === 0 ? (
              <p className="text-sm text-muted-foreground">No drafts yet.</p>
            ) : (
              <ul className="max-h-56 divide-y overflow-y-auto pr-1">
                {emails.slice(0, 5).map((email) => (
                  <li key={email.id} className="flex items-center gap-2 py-2 text-sm first:pt-0 last:pb-0">
                    <Mail className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                    <span className="min-w-0 flex-1 truncate">
                      {email.subject || "(no subject)"}
                    </span>
                    <Badge
                      variant={
                        email.status === "approved"
                          ? "default"
                          : email.status === "discarded"
                            ? "destructive"
                            : "secondary"
                      }
                    >
                      {email.status}
                    </Badge>
                    <span className="shrink-0 text-xs text-muted-foreground">
                      {formatWhen(email.createdAt)}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>
      </div>

      <Card className="mt-6">
        <CardHeader>
          <CardDescription>System</CardDescription>
        </CardHeader>
        <CardContent className="space-y-2 text-sm">
          {error && !info ? (
            <p className="flex items-center gap-2 text-destructive">
              <XCircle className="h-4 w-4" /> Could not reach the backend: {error}
            </p>
          ) : info ? (
            <>
              <p className="flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 text-emerald-600" />
                Database connected · schema v{info.schemaVersion} · app v{info.appVersion}
              </p>
              <code className="block truncate rounded-md bg-muted px-3 py-1.5 text-xs">
                {info.dbPath}
              </code>
            </>
          ) : (
            <Skeleton className="h-5 w-64" />
          )}
        </CardContent>
      </Card>
    </section>
  );
}
