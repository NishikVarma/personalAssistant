import type { LucideIcon } from "lucide-react";
import {
  BellRing,
  Briefcase,
  FileText,
  LayoutDashboard,
  Mail,
  Settings as SettingsIcon,
  UserRound,
  Users,
} from "lucide-react";

export interface SectionMeta {
  path: string;
  label: string;
  icon: LucideIcon;
  phase?: number;
}

export const SECTIONS: SectionMeta[] = [
  { path: "/", label: "Dashboard", icon: LayoutDashboard },
  { path: "/applications", label: "Applications", icon: Briefcase, phase: 5 },
  { path: "/contacts", label: "Contacts", icon: Users, phase: 4 },
  { path: "/emails", label: "Emails", icon: Mail, phase: 7 },
  { path: "/follow-ups", label: "Follow-ups", icon: BellRing, phase: 10 },
  { path: "/resumes", label: "Resumes", icon: FileText, phase: 11 },
  { path: "/career-profile", label: "Career Profile", icon: UserRound, phase: 3 },
  { path: "/settings", label: "Settings", icon: SettingsIcon },
];
