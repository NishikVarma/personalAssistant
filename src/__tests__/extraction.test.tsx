import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import ExtractionReview from "@/components/resumes/ExtractionReview";
import Resumes from "@/pages/Resumes";

const EXTRACTED = {
  fullName: "Nishik Varma",
  email: "nishik@example.com",
  phone: "",
  location: "India",
  summary: "Backend engineer",
  education: [
    {
      institution: "IIT Hyderabad",
      degree: "B.Tech",
      fieldOfStudy: "CS",
      startDate: "2022-08",
      endDate: null,
      grade: null,
      location: null,
      details: "",
    },
  ],
  experience: [],
  projects: [],
  skills: [{ name: "Rust", category: "language" }],
  certifications: [],
  achievements: [],
  links: [],
};

const COUNTS = {
  identityUpdated: true,
  education: 1,
  experience: 0,
  projects: 0,
  skills: 1,
  certifications: 0,
  achievements: 0,
  links: 0,
  skippedDuplicates: 0,
};

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation((command: string) =>
    command === "profile_import_extracted" ? Promise.resolve(COUNTS) : Promise.resolve(null),
  );
});

afterEach(cleanup);

describe("ExtractionReview", () => {
  it("renders extracted sections with editable fields", () => {
    render(<ExtractionReview profile={EXTRACTED} onClose={() => {}} onImported={() => {}} />);

    expect(screen.getByDisplayValue("Nishik Varma")).toBeTruthy();
    expect(screen.getByDisplayValue("IIT Hyderabad")).toBeTruthy();
    expect(screen.getByDisplayValue("Rust")).toBeTruthy();
    expect(screen.getByText(/import 3 items/i)).toBeTruthy();
  });

  it("excludes unticked items from the import payload", async () => {
    const user = userEvent.setup();
    const onImported = vi.fn();
    render(<ExtractionReview profile={EXTRACTED} onClose={() => {}} onImported={onImported} />);

    await user.click(screen.getByLabelText(/include rust/i));
    await user.click(screen.getByRole("button", { name: /import 2 items/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "profile_import_extracted",
        expect.objectContaining({
          profile: expect.objectContaining({
            fullName: "Nishik Varma",
            skills: [],
            education: [expect.objectContaining({ institution: "IIT Hyderabad" })],
          }),
          markVerified: true,
        }),
      );
    });
    expect(onImported).toHaveBeenCalledWith(COUNTS);
  });
});

describe("Resumes extraction flow", () => {
  const PDF = {
    id: 1,
    kind: "pdf_master",
    originalFilename: "Resume.pdf",
    storedPath: "/data/resumes/x.pdf",
    sha256: "abc",
    fileSize: 1024,
    notes: "",
    createdAt: "2026-08-26T10:00:00Z",
    updatedAt: "2026-08-26T10:00:00Z",
  };

  it("opens the review dialog after extraction", async () => {
    invokeMock.mockImplementation((command: string) =>
      command === "resume_file_list"
        ? Promise.resolve([PDF])
        : command === "latex_detect"
          ? Promise.resolve({ available: false, engine: null })
          : command === "resume_extract_profile"
            ? Promise.resolve(EXTRACTED)
            : Promise.resolve(null),
    );
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <Resumes />
      </MemoryRouter>,
    );

    await user.click(await screen.findByRole("button", { name: /extract profile/i }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("resume_extract_profile", { id: 1 });
      expect(screen.getByText("Review extracted profile")).toBeTruthy();
    });
  });

  it("offers the paste fallback when the pdf has no text layer", async () => {
    invokeMock.mockImplementation((command: string) =>
      command === "resume_file_list"
        ? Promise.resolve([PDF])
        : command === "latex_detect"
          ? Promise.resolve({ available: false, engine: null })
          : command === "resume_extract_profile"
            ? Promise.reject(new Error("this PDF has no readable text layer (likely a scan). Use the paste option instead."))
            : Promise.resolve(null),
    );
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <Resumes />
      </MemoryRouter>,
    );

    await user.click(await screen.findByRole("button", { name: /extract profile/i }));
    expect(
      await screen.findByText(/paste the resume content/i, {}, { timeout: 3000 }),
    ).toBeTruthy();
  });
});
