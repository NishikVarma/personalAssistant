import { useEffect, useState } from "react";
import { CheckCircle2, Loader2, XCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ipc, type AppInfo } from "@/lib/ipc";

export default function Settings() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [dbError, setDbError] = useState<string | null>(null);

  const [key, setKey] = useState("greeting");
  const [value, setValue] = useState("");
  const [status, setStatus] = useState<{ kind: "ok" | "err" | "busy"; msg: string } | null>(null);

  const loadInfo = () => {
    setDbError(null);
    ipc
      .appInfo()
      .then(setInfo)
      .catch((e) => setDbError(String(e)));
  };

  useEffect(loadInfo, []);

  const save = async () => {
    setStatus({ kind: "busy", msg: "Saving…" });
    try {
      await ipc.setSetting(key.trim(), value);
      setStatus({ kind: "ok", msg: `Saved "${key.trim()}"` });
    } catch (e) {
      setStatus({ kind: "err", msg: String(e) });
    }
  };

  const load = async () => {
    setStatus({ kind: "busy", msg: "Loading…" });
    try {
      const stored = await ipc.getSetting(key.trim());
      setValue(stored ?? "");
      setStatus({
        kind: "ok",
        msg: stored === null ? `No value for "${key.trim()}"` : `Loaded value`,
      });
    } catch (e) {
      setStatus({ kind: "err", msg: String(e) });
    }
  };

  return (
    <section className="max-w-3xl">
      <header className="mb-8">
        <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Connection status and local preferences. Secrets move to the OS keychain in Phase 8.
        </p>
      </header>

      <div className="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>Database</CardTitle>
            <CardDescription>
              SQLite database created and migrated automatically on startup.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3 text-sm">
            {dbError ? (
              <p className="flex items-center gap-2 text-destructive">
                <XCircle className="h-4 w-4" /> {dbError}
                <Button size="sm" variant="outline" onClick={loadInfo}>
                  Retry
                </Button>
              </p>
            ) : (
              info && (
                <>
                  <p className="flex items-center gap-2">
                    <CheckCircle2 className="h-4 w-4 text-emerald-600" />
                    Connected · schema v{info.schemaVersion} · app v{info.appVersion}
                  </p>
                  <code className="block truncate rounded-md bg-muted px-3 py-2 text-xs">
                    {info.dbPath}
                  </code>
                </>
              )
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Key-value store</CardTitle>
            <CardDescription>
              Round-trip check of the IPC bridge into the settings table.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="kv-key">Key</Label>
                <Input
                  id="kv-key"
                  value={key}
                  onChange={(e) => setKey(e.target.value)}
                  placeholder="e.g. follow_up_days"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="kv-value">Value</Label>
                <Input
                  id="kv-value"
                  value={value}
                  onChange={(e) => setValue(e.target.value)}
                  placeholder="stored value"
                />
              </div>
            </div>
            <div className="flex items-center gap-3">
              <Button onClick={save}>Save</Button>
              <Button variant="outline" onClick={load}>
                Load
              </Button>
              {status && (
                <span
                  className={`flex items-center gap-1.5 text-xs ${
                    status.kind === "err"
                      ? "text-destructive"
                      : status.kind === "ok"
                        ? "text-emerald-600"
                        : "text-muted-foreground"
                  }`}
                >
                  {status.kind === "busy" && <Loader2 className="h-3 w-3 animate-spin" />}
                  {status.msg}
                </span>
              )}
            </div>
          </CardContent>
        </Card>
      </div>
    </section>
  );
}
