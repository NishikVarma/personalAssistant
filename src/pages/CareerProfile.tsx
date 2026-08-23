import AchievementsSection from "@/components/profile/sections/AchievementsSection";
import CertificationsSection from "@/components/profile/sections/CertificationsSection";
import EducationSection from "@/components/profile/sections/EducationSection";
import ExperienceSection from "@/components/profile/sections/ExperienceSection";
import IdentitySection from "@/components/profile/sections/IdentitySection";
import LinksSection from "@/components/profile/sections/LinksSection";
import ProjectsSection from "@/components/profile/sections/ProjectsSection";
import SkillsSection from "@/components/profile/sections/SkillsSection";

export default function CareerProfile() {
  return (
    <section>
      <header className="mb-8">
        <h1 className="text-2xl font-semibold tracking-tight">Career Profile</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          The verified source of truth for every generated email, application and resume. Mark
          entries as verified only when they are accurate — the AI never claims anything that is
          not here.
        </p>
      </header>

      <div className="space-y-6">
        <IdentitySection />
        <div className="grid gap-6 lg:grid-cols-2">
          <EducationSection />
          <SkillsSection />
        </div>
        <ExperienceSection />
        <ProjectsSection />
        <CertificationsSection />
        <AchievementsSection />
        <LinksSection />
      </div>
    </section>
  );
}
