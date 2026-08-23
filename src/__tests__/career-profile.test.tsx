import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn().mockResolvedValue(undefined),
}));

import CareerProfile from "@/pages/CareerProfile";

const PROFILE = {
  id: 1,
  fullName: "Nishik Varma",
  email: "nishik@example.com",
  phone: "",
  location: "",
  summary: "",
  verified: true,
  createdAt: "2026-08-23T00:00:00Z",
  updatedAt: "2026-08-23T00:00:00Z",
};

const EDUCATION = {
  id: 1,
  institution: "IIT Hyderabad",
  degree: "B.Tech",
  fieldOfStudy: "Computer Science",
  startDate: "2022-08-01",
  endDate: null,
  grade: null,
  location: null,
  details: "",
  verified: false,
  createdAt: "2026-08-23T00:00:00Z",
  updatedAt: "2026-08-23T00:00:00Z",
};

const SKILLS = [
  { id: 1, name: "Rust", category: "language", createdAt: "2026-08-23T00:00:00Z" },
  { id: 2, name: "React", category: "framework", createdAt: "2026-08-23T00:00:00Z" },
];

function stubInvoke(command: string, args?: Record<string, unknown>) {
  switch (command) {
    case "profile_get":
      return PROFILE;
    case "profile_update":
      return { ...PROFILE, ...(args?.input as object) };
    case "education_list":
      return [EDUCATION];
    case "skill_list":
      return SKILLS;
    default:
      return [];
  }
}

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) =>
    Promise.resolve(stubInvoke(command, args)),
  );
});

afterEach(cleanup);

describe("CareerProfile", () => {
  it("renders every profile section", async () => {
    render(<CareerProfile />);

    for (const section of [
      "Identity",
      "Education",
      "Experience",
      "Projects",
      "Skills",
      "Certifications",
      "Achievements",
      "Links",
    ]) {
      expect(screen.getByText(section)).toBeTruthy();
    }
  });

  it("loads and shows stored entities through IPC", async () => {
    render(<CareerProfile />);

    await waitFor(() => {
      expect(screen.getByText("IIT Hyderabad")).toBeTruthy();
    });
    expect(await screen.findByText("Rust")).toBeTruthy();
    expect(screen.getByDisplayValue("Nishik Varma")).toBeTruthy();
    expect(invokeMock).toHaveBeenCalledWith("profile_get");
    expect(invokeMock).toHaveBeenCalledWith("education_list");
    expect(invokeMock).toHaveBeenCalledWith("skill_list");
  });

  it("opens the add-education dialog with required-field validation", async () => {
    const user = userEvent.setup();
    render(<CareerProfile />);

    const addButtons = screen.getAllByRole("button", { name: /add/i });
    await user.click(addButtons[0]);

    const dialog = await screen.findByRole("dialog");
    expect(dialog).toBeTruthy();

    // required institution blocks submit
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Institution is required.")).toBeTruthy();

    await user.type(screen.getByLabelText(/institution/i), "NIT Warangal");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "education_create",
        expect.objectContaining({
          input: expect.objectContaining({ institution: "NIT Warangal" }),
        }),
      );
    });
  });

  it("toggles the identity verified flag", async () => {
    const user = userEvent.setup();
    render(<CareerProfile />);

    // the profile is the only verified entity in the stub, so exactly one toggle offers un-verifying
    const toggles = await screen.findAllByTitle("Mark as unverified");
    expect(toggles.length).toBe(1);
    await user.click(toggles[0]);
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("profile_set_verified", { verified: false });
    });
  });
});
