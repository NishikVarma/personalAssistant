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
  /** Overrides the placeholder based on current form values (e.g. custom link kinds). */
  dynamicPlaceholder?: (values: FormValues) => string | undefined;
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

  const advanceFocus = (current: HTMLElement | null) => {
    const form = current?.closest("form");
    if (!form || !current) return;
    const focusables = Array.from(
      form.querySelectorAll<HTMLElement>("input, select, textarea"),
    ).filter((el) => !el.hasAttribute("disabled"));
    const next = focusables[focusables.indexOf(current) + 1];
    if (next) next.focus();
  };

  const handleDateKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      advanceFocus(e.currentTarget);
    }
  };

  const handleDateChange = (field: FieldDef, e: React.ChangeEvent<HTMLInputElement>) => {
    setValue(field.name, e.target.value);
    // a complete date means the picker was used — move on instead of trapping focus
    if (/^\d{4}-\d{2}-\d{2}$/.test(e.target.value)) {
      advanceFocus(e.currentTarget);
    }
  };

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
                  {field.required ? (
                    <span className="text-destructive">*</span>
                  ) : field.type === "date" ? (
                    <span className="text-xs font-normal text-muted-foreground">
                      (optional)
                    </span>
                  ) : null}
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
                    placeholder={
                      field.dynamicPlaceholder?.(values) ?? field.placeholder
                    }
                    onChange={(e) =>
                      field.type === "date"
                        ? handleDateChange(field, e)
                        : setValue(field.name, e.target.value)
                    }
                    onKeyDown={field.type === "date" ? handleDateKeyDown : undefined}
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
