import { useEffect, useState } from "react";
import { CheckCircle2, XCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { ipc, type AppInfo } from "@/lib/ipc";

export default function Dashboard() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    ipc
      .appInfo()
      .then(setInfo)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <section>
      <header className="mb-8">
        <h1 className="text-2xl font-semibold tracking-tight">Dashboard</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Local-first job application copilot. Everything below runs on your machine.
        </p>
      </header>

      {error ? (
        <Card className="border-destructive">
          <CardContent className="pt-6 text-sm text-destructive">
            Failed to reach backend: {error}
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          <Card>
            <CardHeader>
              <CardDescription>Database</CardDescription>
              <CardTitle className="flex items-center gap-2">
                {info ? (
                  <>
                    <CheckCircle2 className="h-5 w-5 text-emerald-600" /> Connected
                  </>
                ) : (
                  <>
                    <XCircle className="h-5 w-5 animate-pulse" /> Connecting…
                  </>
                )}
              </CardTitle>
            </CardHeader>
            <CardContent className="text-sm text-muted-foreground">
              SQLite via SQLx, migrations applied automatically at startup.
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardDescription>Schema version</CardDescription>
              <CardTitle>
                {info ? <Badge variant="secondary">v{info.schemaVersion}</Badge> : "…"}
              </CardTitle>
            </CardHeader>
            <CardContent className="text-sm text-muted-foreground">
              Normalized schema covering profile, applications, contacts, emails and resumes.
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardDescription>App version</CardDescription>
              <CardTitle>{info ? `v${info.appVersion}` : "…"}</CardTitle>
            </CardHeader>
            <CardContent className="text-sm text-muted-foreground">
              Session 1 of the build plan: scaffold + database layer complete.
            </CardContent>
          </Card>

          <Card className="sm:col-span-2 lg:col-span-3">
            <CardHeader>
              <CardDescription>Data location</CardDescription>
            </CardHeader>
            <CardContent>
              <code className="block truncate rounded-md bg-muted px-3 py-2 text-xs">
                {info?.dbPath ?? "…"}
              </code>
            </CardContent>
          </Card>
        </div>
      )}
    </section>
  );
}
