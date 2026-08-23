import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  appVersion: string;
  dbPath: string;
  schemaVersion: number;
}

export type ProfileEntityType = "project" | "experience";

export interface UserProfile {
  id: number;
  fullName: string;
  email: string;
  phone: string;
  location: string;
  summary: string;
  verified: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface UserProfileInput {
  fullName: string;
  email: string;
  phone: string;
  location: string;
  summary: string;
}

export interface Education {
  id: number;
  institution: string;
  degree: string;
  fieldOfStudy: string;
  startDate: string | null;
  endDate: string | null;
  grade: string | null;
  location: string | null;
  details: string;
  verified: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface EducationInput {
  institution: string;
  degree: string;
  fieldOfStudy: string;
  startDate: string | null;
  endDate: string | null;
  grade: string | null;
  location: string | null;
  details: string;
}

export type EmploymentType =
  | "internship"
  | "full_time"
  | "part_time"
  | "contract"
  | "freelance";

export interface Experience {
  id: number;
  organization: string;
  title: string;
  employmentType: EmploymentType;
  location: string | null;
  startDate: string | null;
  endDate: string | null;
  currentlyWorking: boolean;
  description: string;
  verified: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ExperienceInput {
  organization: string;
  title: string;
  employmentType: EmploymentType;
  location: string | null;
  startDate: string | null;
  endDate: string | null;
  currentlyWorking: boolean;
  description: string;
}

export type ProjectStatus = "ongoing" | "completed" | "archived";

export interface Project {
  id: number;
  name: string;
  description: string;
  repoUrl: string | null;
  liveUrl: string | null;
  status: ProjectStatus;
  startedOn: string | null;
  endedOn: string | null;
  verified: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ProjectInput {
  name: string;
  description: string;
  repoUrl: string | null;
  liveUrl: string | null;
  status: ProjectStatus;
  startedOn: string | null;
  endedOn: string | null;
}

export type SkillCategory =
  | "language"
  | "framework"
  | "tool"
  | "database"
  | "cloud"
  | "soft_skill"
  | "other";

export interface Skill {
  id: number;
  name: string;
  category: SkillCategory;
  createdAt: string;
}

export interface SkillInput {
  name: string;
  category: SkillCategory;
}

export interface Bullet {
  id: number;
  entityType: ProfileEntityType;
  entityId: number;
  content: string;
  verified: boolean;
  displayOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface BulletInput {
  content: string;
  displayOrder: number;
}

export interface Certification {
  id: number;
  name: string;
  issuer: string;
  issueDate: string | null;
  expiryDate: string | null;
  credentialUrl: string | null;
  verified: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CertificationInput {
  name: string;
  issuer: string;
  issueDate: string | null;
  expiryDate: string | null;
  credentialUrl: string | null;
}

export interface Achievement {
  id: number;
  title: string;
  description: string;
  date: string | null;
  verified: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface AchievementInput {
  title: string;
  description: string;
  date: string | null;
}

export type LinkKind = "linkedin" | "github" | "portfolio" | "other";

export interface Link {
  id: number;
  label: string;
  url: string;
  kind: LinkKind;
  createdAt: string;
}

export interface LinkInput {
  label: string;
  url: string;
  kind: LinkKind;
}

export interface Contact {
  id: number;
  name: string;
  email: string;
  organization: string | null;
  roleTitle: string | null;
  linkedinUrl: string | null;
  notes: string;
  lastContactedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ContactInput {
  name: string;
  email: string;
  organization: string | null;
  roleTitle: string | null;
  linkedinUrl: string | null;
  notes: string;
}

export interface Tag {
  id: number;
  name: string;
  color: string | null;
}

export interface TagInput {
  name: string;
  color: string | null;
}

export type ApplicationStatus =
  | "saved"
  | "preparing"
  | "applied"
  | "contacted"
  | "follow_up_due"
  | "response_received"
  | "oa"
  | "interview"
  | "offer"
  | "rejected"
  | "withdrawn";

export const APPLICATION_STATUSES: ApplicationStatus[] = [
  "saved",
  "preparing",
  "applied",
  "contacted",
  "follow_up_due",
  "response_received",
  "oa",
  "interview",
  "offer",
  "rejected",
  "withdrawn",
];

export interface Application {
  id: number;
  company: string;
  role: string;
  jobDescription: string;
  jobUrl: string | null;
  source: string | null;
  status: ApplicationStatus;
  dateDiscovered: string | null;
  dateApplied: string | null;
  followUpDate: string | null;
  interviewStatus: string | null;
  priority: number;
  notes: string;
  createdAt: string;
  updatedAt: string;
}

export interface ApplicationInput {
  company: string;
  role: string;
  jobDescription: string;
  jobUrl: string | null;
  source: string | null;
  dateDiscovered: string | null;
  dateApplied: string | null;
  followUpDate: string | null;
  interviewStatus: string | null;
  priority: number;
  notes: string;
}

export interface AiConfig {
  model: string;
  hasApiKey: boolean;
}

export interface AiTestResult {
  ok: boolean;
  latencyMs: number | null;
  error: string | null;
  model: string;
}

export type EmailType =
  | "cold_outreach"
  | "job_application"
  | "referral_request"
  | "follow_up"
  | "internship_inquiry"
  | "application_status";

export const EMAIL_TYPES: EmailType[] = [
  "cold_outreach",
  "job_application",
  "referral_request",
  "follow_up",
  "internship_inquiry",
  "application_status",
];

export type EmailStatus = "draft" | "edited" | "approved" | "sent" | "discarded";

export interface GeneratedEmail {
  id: number;
  applicationId: number | null;
  contactId: number | null;
  emailType: EmailType;
  subject: string | null;
  body: string;
  provider: string | null;
  model: string | null;
  status: EmailStatus;
  createdAt: string;
  updatedAt: string;
}

export interface GeneratedEmailInput {
  applicationId: number | null;
  contactId: number | null;
  emailType: EmailType;
  subject: string | null;
  body: string;
}

export interface EmailDraftRequest {
  recipientEmail: string;
  recipientName: string | null;
  company: string | null;
  role: string | null;
  jobDescription: string | null;
  additionalContext: string | null;
  emailType: EmailType;
  applicationId: number | null;
  contactId: number | null;
}

export interface ExtractedContact {
  name: string | null;
  organization: string | null;
}

export const ipc = {
  appInfo: () => invoke<AppInfo>("get_app_info"),
  getSetting: (key: string) => invoke<string | null>("get_setting", { key }),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
  deleteSetting: (key: string) => invoke<boolean>("delete_setting", { key }),

  profile: {
    get: () => invoke<UserProfile>("profile_get"),
    update: (input: UserProfileInput) => invoke<UserProfile>("profile_update", { input }),
    setVerified: (verified: boolean) => invoke<void>("profile_set_verified", { verified }),
  },

  education: {
    list: () => invoke<Education[]>("education_list"),
    create: (input: EducationInput) => invoke<Education>("education_create", { input }),
    update: (id: number, input: EducationInput) =>
      invoke<Education>("education_update", { id, input }),
    setVerified: (id: number, verified: boolean) =>
      invoke<void>("education_set_verified", { id, verified }),
    remove: (id: number) => invoke<boolean>("education_delete", { id }),
  },

  experience: {
    list: () => invoke<Experience[]>("experience_list"),
    create: (input: ExperienceInput) => invoke<Experience>("experience_create", { input }),
    update: (id: number, input: ExperienceInput) =>
      invoke<Experience>("experience_update", { id, input }),
    setVerified: (id: number, verified: boolean) =>
      invoke<void>("experience_set_verified", { id, verified }),
    remove: (id: number) => invoke<boolean>("experience_delete", { id }),
  },

  project: {
    list: () => invoke<Project[]>("project_list"),
    create: (input: ProjectInput) => invoke<Project>("project_create", { input }),
    update: (id: number, input: ProjectInput) => invoke<Project>("project_update", { id, input }),
    setVerified: (id: number, verified: boolean) =>
      invoke<void>("project_set_verified", { id, verified }),
    remove: (id: number) => invoke<boolean>("project_delete", { id }),
  },

  skill: {
    list: () => invoke<Skill[]>("skill_list"),
    create: (input: SkillInput) => invoke<Skill>("skill_create", { input }),
    update: (id: number, input: SkillInput) => invoke<Skill>("skill_update", { id, input }),
    remove: (id: number) => invoke<boolean>("skill_delete", { id }),
    listForEntity: (entityType: ProfileEntityType, entityId: number) =>
      invoke<Skill[]>("skill_list_for_entity", { entityType, entityId }),
    replaceForEntity: (entityType: ProfileEntityType, entityId: number, skillIds: number[]) =>
      invoke<void>("skill_replace_for_entity", { entityType, entityId, skillIds }),
  },

  bullet: {
    listForEntity: (entityType: ProfileEntityType, entityId: number) =>
      invoke<Bullet[]>("bullet_list_for_entity", { entityType, entityId }),
    create: (entityType: ProfileEntityType, entityId: number, input: BulletInput) =>
      invoke<Bullet>("bullet_create", { entityType, entityId, input }),
    update: (id: number, input: BulletInput) => invoke<Bullet>("bullet_update", { id, input }),
    setVerified: (id: number, verified: boolean) =>
      invoke<void>("bullet_set_verified", { id, verified }),
    remove: (id: number) => invoke<boolean>("bullet_delete", { id }),
  },

  certification: {
    list: () => invoke<Certification[]>("certification_list"),
    create: (input: CertificationInput) => invoke<Certification>("certification_create", { input }),
    update: (id: number, input: CertificationInput) =>
      invoke<Certification>("certification_update", { id, input }),
    setVerified: (id: number, verified: boolean) =>
      invoke<void>("certification_set_verified", { id, verified }),
    remove: (id: number) => invoke<boolean>("certification_delete", { id }),
  },

  achievement: {
    list: () => invoke<Achievement[]>("achievement_list"),
    create: (input: AchievementInput) => invoke<Achievement>("achievement_create", { input }),
    update: (id: number, input: AchievementInput) =>
      invoke<Achievement>("achievement_update", { id, input }),
    setVerified: (id: number, verified: boolean) =>
      invoke<void>("achievement_set_verified", { id, verified }),
    remove: (id: number) => invoke<boolean>("achievement_delete", { id }),
  },

  link: {
    list: () => invoke<Link[]>("link_list"),
    create: (input: LinkInput) => invoke<Link>("link_create", { input }),
    update: (id: number, input: LinkInput) => invoke<Link>("link_update", { id, input }),
    remove: (id: number) => invoke<boolean>("link_delete", { id }),
  },

  contact: {
    list: (search = "") => invoke<Contact[]>("contact_list", { search }),
    create: (input: ContactInput) => invoke<Contact>("contact_create", { input }),
    update: (id: number, input: ContactInput) => invoke<Contact>("contact_update", { id, input }),
    setLastContacted: (id: number, lastContactedAt: string | null) =>
      invoke<void>("contact_set_last_contacted", { id, lastContactedAt }),
    remove: (id: number) => invoke<boolean>("contact_delete", { id }),
    listTags: (contactId: number) => invoke<Tag[]>("contact_list_tags", { contactId }),
    replaceTags: (contactId: number, tagIds: number[]) =>
      invoke<void>("contact_replace_tags", { contactId, tagIds }),
  },

  tag: {
    list: () => invoke<Tag[]>("tag_list"),
    create: (input: TagInput) => invoke<Tag>("tag_create", { input }),
    remove: (id: number) => invoke<boolean>("tag_delete", { id }),
  },

  application: {
    list: (status?: ApplicationStatus | null) =>
      invoke<Application[]>("application_list", { status: status ?? undefined }),
    create: (input: ApplicationInput) => invoke<Application>("application_create", { input }),
    update: (id: number, input: ApplicationInput) =>
      invoke<Application>("application_update", { id, input }),
    setStatus: (id: number, status: ApplicationStatus) =>
      invoke<Application>("application_set_status", { id, status }),
    remove: (id: number) => invoke<boolean>("application_delete", { id }),
  },

  ai: {
    getConfig: () => invoke<AiConfig>("ai_get_config"),
    setModel: (model: string) => invoke<void>("ai_set_model", { model }),
    setApiKey: (apiKey: string) => invoke<void>("ai_set_api_key", { apiKey }),
    clearApiKey: () => invoke<boolean>("ai_clear_api_key"),
    testConnection: () => invoke<AiTestResult>("ai_test_connection"),
    generateEmail: (request: EmailDraftRequest) =>
      invoke<GeneratedEmail>("ai_generate_email", { request }),
    extractContact: (email: string) => invoke<ExtractedContact>("ai_extract_contact", { email }),
  },

  generatedEmail: {
    list: (status?: EmailStatus | null) =>
      invoke<GeneratedEmail[]>("generated_email_list", { status: status ?? undefined }),
    get: (id: number) => invoke<GeneratedEmail>("generated_email_get", { id }),
    create: (input: GeneratedEmailInput) =>
      invoke<GeneratedEmail>("generated_email_create", { input }),
    update: (id: number, subject: string | null, body: string) =>
      invoke<GeneratedEmail>("generated_email_update", { id, subject, body }),
    setStatus: (id: number, status: EmailStatus) =>
      invoke<GeneratedEmail>("generated_email_set_status", { id, status }),
    remove: (id: number) => invoke<boolean>("generated_email_delete", { id }),
  },
};
