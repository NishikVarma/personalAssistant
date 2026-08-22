import { cleanup, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import AppLayout from "@/components/AppLayout";
import Placeholder from "@/pages/Placeholder";
import { Briefcase } from "lucide-react";

afterEach(cleanup);

describe("AppLayout", () => {
  it("renders all eight sections in the sidebar", () => {
    render(
      <MemoryRouter initialEntries={["/"]}>
        <Routes>
          <Route element={<AppLayout />}>
            <Route path="/" element={<div>home</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );
    for (const label of [
      "Dashboard",
      "Applications",
      "Contacts",
      "Emails",
      "Follow-ups",
      "Resumes",
      "Career Profile",
      "Settings",
    ]) {
      expect(screen.getAllByText(label).length).toBeGreaterThan(0);
    }
  });
});

describe("Placeholder", () => {
  it("shows the target phase", () => {
    render(<Placeholder title="Applications" phase={5} icon={Briefcase} />);
    expect(screen.getByText("Arrives in Phase 5")).toBeTruthy();
    expect(screen.getByText("Applications")).toBeTruthy();
  });
});
