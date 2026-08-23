import { useEffect, useState } from "react";
import { CheckCircle2, Loader2, XCircle } from "lucide-react";
import DeleteButton from "@/components/profile/DeleteButton";
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
import { ipc, type AiConfig, type AiTestResult, type AppInfo } from "@/lib/ipc";

function AiProviderCard() {
  const [config, setConfig] = useState<AiConfig | null>(null);
  const [model, setModel] = useState("");
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [testResult, setTestResult] = useState<AiTestResult | null>(null);

  useEffect(() => {
    ipc.ai
      .getConfig()
      .then((cfg) => {
        setConfig(cfg);
        setModel(cfg.model);
      })
      .catch((e) => setError(String(e)));
  }, []);

  const reloadConfig = async () => {
    const cfg = await ipc.ai.getConfig();
    setConfig(cfg);
    setModel(cfg.model);
    return cfg;
  };

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      await ipc.ai.setModel(model);
      if (apiKeyInput.trim()) {
        await ipc.ai.setApiKey(apiKeyInput);
        setApiKeyInput("");
      }
      await reloadConfig();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const testConnection = async () => {
    setBusy(true);
    setError(null);
    setTestResult(null);
    try {
      setTestResult(await ipc.ai.testConnection());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>AI provider</CardTitle>
        <CardDescription>
          Gemini API access. The key is stored in your operating system's secure credential
          storage — never in the database.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="ai-model">Model</Label>
            <Input
              id="ai-model"
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder="gemini-2.5-flash"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="ai-key">API key</Label>
            <Input
              id="ai-key"
              type="password"
              value={apiKeyInput}
              onChange={(e) => setApiKeyInput(e.target.value)}
              placeholder={config?.hasApiKey ? "Stored — leave blank to keep" : "Paste API key"}
            />
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button disabled={busy} onClick={() => void save()}>
            {busy ? <Loader2 className="animate-spin" /> : null} Save
          </Button>
          <Button variant="outline" disabled={busy} onClick={() => void testConnection()}>
            Test connection
          </Button>
          {config?.hasApiKey ? (
            <DeleteButton
              confirmLabel="Remove key"
              cancelLabel="Keep key"
              onConfirm={async () => {
                await ipc.ai.clearApiKey();
                setTestResult(null);
                await reloadConfig();
              }}
            />
          ) : null}
          {testResult ? (
            testResult.ok ? (
              <span className="flex items-center gap-1.5 text-xs text-emerald-600">
                <CheckCircle2 className="h-3.5 w-3.5" />
                Connected{testResult.latencyMs != null ? ` (${testResult.latencyMs} ms)` : ""}
              </span>
            ) : (
              <span className="flex items-center gap-1.5 text-xs text-destructive">
                <XCircle className="h-3.5 w-3.5" /> {testResult.error}
              </span>
            )
          ) : null}
          {error ? (
            <span className="text-xs text-destructive">{error}</span>
          ) : null}
        </div>
      </CardContent>
    </Card>
  );
}

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
          AI provider and local preferences. Secrets are stored in the OS keychain.
        </p>
      </header>

      <div className="space-y-6">
        <AiProviderCard />

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
