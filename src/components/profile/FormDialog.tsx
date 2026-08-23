import { useState } from "react";
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
import { Select } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";

export interface FieldOption {
  value: string;
  label: string;
}

export interface FieldDef {
  name: string;
  label: string;
  type: "text" | "textarea" | "date" | "select";
  options?: FieldOption[];
  required?: boolean;
  placeholder?: string;
  full?: boolean;
}

export type FormValues = Record<string, string>;

interface FormDialogProps {
  title: string;
  description?: string;
  fields: FieldDef[];
  initial: FormValues;
  submitLabel?: string;
  onSubmit: (values: FormValues) => Promise<void>;
  onClose: () => void;
}

export function emptyToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}

export default function FormDialog({
  title,
  description,
  fields,
  initial,
  submitLabel = "Save",
  onSubmit,
  onClose,
}: FormDialogProps) {
  const [values, setValues] = useState<FormValues>(() => {
    const base: FormValues = {};
    for (const field of fields) {
      base[field.name] = initial[field.name] ?? "";
    }
    return base;
  });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const setValue = (name: string, value: string) =>
    setValues((prev) => ({ ...prev, [name]: value }));

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    for (const field of fields) {
      if (field.required && values[field.name].trim() === "") {
        setError(`${field.label} is required.`);
        return;
      }
    }
    setBusy(true);
    setError(null);
    try {
      await onSubmit(values);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {description ? <DialogDescription>{description}</DialogDescription> : null}
        </DialogHeader>
        <form className="grid gap-4" onSubmit={handleSubmit}>
          <div className="grid gap-4 sm:grid-cols-2">
            {fields.map((field) => (
              <div
                key={field.name}
                className={field.full || field.type === "textarea" ? "sm:col-span-2" : undefined}
              >
                <Label htmlFor={`field-${field.name}`} className="mb-1.5">
                  {field.label}
                  {field.required ? <span className="text-destructive">*</span> : null}
                </Label>
                {field.type === "textarea" ? (
                  <Textarea
                    id={`field-${field.name}`}
                    value={values[field.name]}
                    placeholder={field.placeholder}
                    onChange={(e) => setValue(field.name, e.target.value)}
                  />
                ) : field.type === "select" ? (
                  <Select
                    id={`field-${field.name}`}
                    value={values[field.name]}
                    onChange={(e) => setValue(field.name, e.target.value)}
                  >
                    {(field.options ?? []).map((opt) => (
                      <option key={opt.value} value={opt.value}>
                        {opt.label}
                      </option>
                    ))}
                  </Select>
                ) : (
                  <Input
                    id={`field-${field.name}`}
                    type={field.type === "date" ? "date" : "text"}
                    value={values[field.name]}
                    placeholder={field.placeholder}
                    onChange={(e) => setValue(field.name, e.target.value)}
                  />
                )}
              </div>
            ))}
          </div>
          {error ? <p className="text-sm text-destructive">{error}</p> : null}
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose} disabled={busy}>
              Cancel
            </Button>
            <Button type="submit" disabled={busy}>
              {busy ? "Saving…" : submitLabel}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
