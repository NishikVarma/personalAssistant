import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import AchievementsSection from "@/components/profile/sections/AchievementsSection";
import CertificationsSection from "@/components/profile/sections/CertificationsSection";
import EducationSection from "@/components/profile/sections/EducationSection";
import ExperienceSection from "@/components/profile/sections/ExperienceSection";
import IdentitySection from "@/components/profile/sections/IdentitySection";
import LinksSection from "@/components/profile/sections/LinksSection";
import ProjectsSection from "@/components/profile/sections/ProjectsSection";
import SkillsSection from "@/components/profile/sections/SkillsSection";

const SECTION_LINKS = [
  { id: "identity", label: "Identity" },
  { id: "education", label: "Education" },
  { id: "skills", label: "Skills" },
  { id: "experience", label: "Experience" },
  { id: "projects", label: "Projects" },
  { id: "certifications", label: "Certifications" },
  { id: "achievements", label: "Achievements" },
  { id: "links", label: "Links" },
];

const SECTION_IDS = SECTION_LINKS.map((s) => s.id);

function useActiveSection(): string {
  const [active, setActive] = useState(SECTION_IDS[0]);

  useEffect(() => {
    if (typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) setActive(entry.target.id);
        }
      },
      { rootMargin: "-80px 0px -55% 0px" },
    );
    for (const id of SECTION_IDS) {
      const el = document.getElementById(id);
      if (el) observer.observe(el);
    }
    return () => observer.disconnect();
  }, []);

  return active;
}

export default function CareerProfile() {
  const active = useActiveSection();

  const scrollTo = (id: string) => {
    document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" });
  };

  return (
    <section>
      <header className="mb-4">
        <h1 className="text-2xl font-semibold tracking-tight">Career Profile</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          The verified source of truth for every generated email, application and resume. Mark
          entries as verified only when they are accurate — the AI never claims anything that is
          not here.
        </p>
      </header>

      <nav
        aria-label="Profile sections"
        className="sticky top-0 z-10 -mx-2 mb-6 flex gap-1 overflow-x-auto rounded-lg border border-border bg-background/95 px-2 py-1.5 backdrop-blur"
      >
        {SECTION_LINKS.map(({ id, label }) => (
          <a
            key={id}
            href={`#${id}`}
            onClick={(e) => {
              e.preventDefault();
              scrollTo(id);
            }}
            className={cn(
              "whitespace-nowrap rounded-md px-3 py-1 text-xs font-medium transition-colors",
              active === id
                ? "bg-accent text-accent-foreground"
                : "text-muted-foreground hover:bg-accent/50 hover:text-foreground",
            )}
          >
            {label}
          </a>
        ))}
      </nav>

      <div className="space-y-6">
        <div id="identity" className="scroll-mt-24">
          <IdentitySection />
        </div>
        <div className="grid gap-6 lg:grid-cols-2">
          <div id="education" className="scroll-mt-24">
            <EducationSection />
          </div>
          <div id="skills" className="scroll-mt-24">
            <SkillsSection />
          </div>
        </div>
        <div id="experience" className="scroll-mt-24">
          <ExperienceSection />
        </div>
        <div id="projects" className="scroll-mt-24">
          <ProjectsSection />
        </div>
        <div id="certifications" className="scroll-mt-24">
          <CertificationsSection />
        </div>
        <div id="achievements" className="scroll-mt-24">
          <AchievementsSection />
        </div>
        <div id="links" className="scroll-mt-24">
          <LinksSection />
        </div>
      </div>
    </section>
  );
}
