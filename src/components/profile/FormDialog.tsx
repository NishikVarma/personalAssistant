import { useState } from "react";
import { CalendarPlus, X } from "lucide-react";
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
  /** Overrides required-ness based on current form values (e.g. link labels). */
  dynamicRequired?: (values: FormValues) => boolean;
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

const MONTHS: { value: string; label: string }[] = [
  { value: "01", label: "January" },
  { value: "02", label: "February" },
  { value: "03", label: "March" },
  { value: "04", label: "April" },
  { value: "05", label: "May" },
  { value: "06", label: "June" },
  { value: "07", label: "July" },
  { value: "08", label: "August" },
  { value: "09", label: "September" },
  { value: "10", label: "October" },
  { value: "11", label: "November" },
  { value: "12", label: "December" },
];

function yearOptions(): string[] {
  const current = new Date().getFullYear();
  const years: string[] = [];
  for (let y = current + 5; y >= current - 60; y--) {
    years.push(String(y));
  }
  return years;
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
  // date fields the user opted into but has not completed yet
  const [addedDates, setAddedDates] = useState<Set<string>>(new Set());
  const [dateParts, setDateParts] = useState<Record<string, { year: string; month: string }>>({});
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const setValue = (name: string, value: string) =>
    setValues((prev) => ({ ...prev, [name]: value }));

  const isRequired = (field: FieldDef) =>
    field.dynamicRequired ? field.dynamicRequired(values) : Boolean(field.required);

  const partsFor = (field: FieldDef): { year: string; month: string } => {
    const existing = dateParts[field.name];
    if (existing) return existing;
    const value = values[field.name];
    if (/^\d{4}-\d{2}/.test(value)) {
      return { year: value.slice(0, 4), month: value.slice(5, 7) };
    }
    return { year: String(new Date().getFullYear()), month: "" };
  };

  const revealDate = (field: FieldDef) => {
    setAddedDates((prev) => new Set(prev).add(field.name));
    setDateParts((prev) => ({
      ...prev,
      [field.name]: partsFor(field).month
        ? partsFor(field)
        : { year: String(new Date().getFullYear()), month: "" },
    }));
  };

  const removeDate = (field: FieldDef) => {
    setAddedDates((prev) => {
      const next = new Set(prev);
      next.delete(field.name);
      return next;
    });
    setDateParts((prev) => {
      const next = { ...prev };
      delete next[field.name];
      return next;
    });
    setValue(field.name, "");
  };

  const setDatePart = (field: FieldDef, part: "year" | "month", v: string) => {
    const current = partsFor(field);
    const next = { ...current, [part]: v };
    setDateParts((prev) => ({ ...prev, [field.name]: next }));
    setValue(field.name, next.year && next.month ? `${next.year}-${next.month}` : "");
  };

  const isRevealed = (field: FieldDef) =>
    values[field.name] !== "" || addedDates.has(field.name);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    for (const field of fields) {
      if (isRequired(field) && values[field.name].trim() === "") {
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
                  {isRequired(field) ? <span className="text-destructive">*</span> : null}
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
                ) : field.type === "date" ? (
                  isRevealed(field) ? (
                    <div className="flex items-center gap-1.5">
                      <Select
                        id={`field-${field.name}`}
                        aria-label={`${field.label} month`}
                        value={partsFor(field).month}
                        onChange={(e) => setDatePart(field, "month", e.target.value)}
                      >
                        <option value="">Month…</option>
                        {MONTHS.map((m) => (
                          <option key={m.value} value={m.value}>
                            {m.label}
                          </option>
                        ))}
                      </Select>
                      <Select
                        aria-label={`${field.label} year`}
                        className="w-28"
                        value={partsFor(field).year}
                        onChange={(e) => setDatePart(field, "year", e.target.value)}
                      >
                        {yearOptions().map((y) => (
                          <option key={y} value={y}>
                            {y}
                          </option>
                        ))}
                      </Select>
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-xs"
                        aria-label={`Remove ${field.label}`}
                        onClick={() => removeDate(field)}
                      >
                        <X />
                      </Button>
                    </div>
                  ) : (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      className="w-full"
                      onClick={() => revealDate(field)}
                    >
                      <CalendarPlus /> Add {field.label.toLowerCase()}
                    </Button>
                  )
                ) : (
                  <Input
                    id={`field-${field.name}`}
                    type="text"
                    value={values[field.name]}
                    placeholder={field.dynamicPlaceholder?.(values) ?? field.placeholder}
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
