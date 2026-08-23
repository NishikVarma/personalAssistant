import type { FieldOption } from "@/components/profile/FormDialog";
import type { EmploymentType, ProjectStatus } from "@/lib/ipc";

export const EMPLOYMENT_TYPE_OPTIONS: { value: EmploymentType; label: string }[] = [
  { value: "internship", label: "Internship" },
  { value: "full_time", label: "Full-time" },
  { value: "part_time", label: "Part-time" },
  { value: "contract", label: "Contract" },
  { value: "freelance", label: "Freelance" },
];

export const PROJECT_STATUS_OPTIONS: { value: ProjectStatus; label: string }[] = [
  { value: "ongoing", label: "Ongoing" },
  { value: "completed", label: "Completed" },
  { value: "archived", label: "Archived" },
];

export const SKILL_CATEGORY_OPTIONS: FieldOption[] = [
  { value: "language", label: "Language" },
  { value: "framework", label: "Framework" },
  { value: "tool", label: "Tool" },
  { value: "database", label: "Database" },
  { value: "cloud", label: "Cloud" },
  { value: "soft_skill", label: "Soft skill" },
  { value: "other", label: "Other" },
];

export const LINK_KIND_OPTIONS: FieldOption[] = [
  { value: "linkedin", label: "LinkedIn" },
  { value: "github", label: "GitHub" },
  { value: "portfolio", label: "Portfolio" },
  { value: "other", label: "Other" },
];

export function labelFor(options: FieldOption[], value: string): string {
  return options.find((opt) => opt.value === value)?.label ?? value;
}

/** Formats a stored date string (YYYY-MM-DD or ISO) as YYYY-MM for compact display. */
export function shortDate(value: string | null | undefined): string {
  if (!value) return "";
  return value.slice(0, 10);
}
