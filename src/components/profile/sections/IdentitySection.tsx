import { useEffect, useState } from "react";
import { Save } from "lucide-react";
import SectionCard from "@/components/profile/SectionCard";
import VerifiedToggle from "@/components/profile/VerifiedToggle";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { ipc, type UserProfile } from "@/lib/ipc";

const EMPTY: UserProfile = {
  id: 1,
  fullName: "",
  email: "",
  phone: "",
  location: "",
  summary: "",
  verified: false,
  createdAt: "",
  updatedAt: "",
};

export default function IdentitySection() {
  const [profile, setProfile] = useState<UserProfile | null>(null);
  const [form, setForm] = useState<UserProfile>(EMPTY);
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    ipc.profile
      .get()
      .then((p) => {
        setProfile(p);
        setForm(p);
      })
      .catch((e) => setStatus(String(e)));
  }, []);

  const set = (patch: Partial<UserProfile>) => setForm((prev) => ({ ...prev, ...patch }));

  const save = async () => {
    setBusy(true);
    setStatus(null);
    try {
      const saved = await ipc.profile.update({
        fullName: form.fullName,
        email: form.email,
        phone: form.phone,
        location: form.location,
        summary: form.summary,
      });
      setProfile(saved);
      setForm(saved);
      setStatus("Saved.");
    } catch (e) {
      setStatus(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SectionCard
      title="Identity"
      description="The verified source of truth every generated email and resume draws from."
      action={
        profile ? (
          <VerifiedToggle
            verified={profile.verified}
            onToggle={async () => {
              await ipc.profile.setVerified(!profile.verified);
              const updated = await ipc.profile.get();
              setProfile(updated);
              setForm(updated);
            }}
          />
        ) : undefined
      }
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <div>
          <Label htmlFor="identity-name" className="mb-1.5">
            Full name
          </Label>
          <Input
            id="identity-name"
            value={form.fullName}
            onChange={(e) => set({ fullName: e.target.value })}
          />
        </div>
        <div>
          <Label htmlFor="identity-email" className="mb-1.5">
            Email
          </Label>
          <Input
            id="identity-email"
            type="email"
            value={form.email}
            onChange={(e) => set({ email: e.target.value })}
          />
        </div>
        <div>
          <Label htmlFor="identity-phone" className="mb-1.5">
            Phone
          </Label>
          <Input
            id="identity-phone"
            value={form.phone}
            onChange={(e) => set({ phone: e.target.value })}
          />
        </div>
        <div>
          <Label htmlFor="identity-location" className="mb-1.5">
            Location
          </Label>
          <Input
            id="identity-location"
            value={form.location}
            onChange={(e) => set({ location: e.target.value })}
          />
        </div>
        <div className="sm:col-span-2">
          <Label htmlFor="identity-summary" className="mb-1.5">
            Summary
          </Label>
          <Textarea
            id="identity-summary"
            placeholder="A short professional summary used in outreach and resumes."
            value={form.summary}
            onChange={(e) => set({ summary: e.target.value })}
          />
        </div>
      </div>
      <div className="mt-4 flex items-center gap-3">
        <Button onClick={save} disabled={busy}>
          <Save /> {busy ? "Saving…" : "Save identity"}
        </Button>
        {status ? (
          <p className={status === "Saved." ? "text-sm text-muted-foreground" : "text-sm text-destructive"}>
            {status}
          </p>
        ) : null}
      </div>
    </SectionCard>
  );
}
