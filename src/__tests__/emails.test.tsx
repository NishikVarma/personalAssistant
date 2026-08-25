import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import Emails from "@/pages/Emails";

const EXISTING = {
  id: 7,
  applicationId: null,
  contactId: null,
  emailType: "cold_outreach",
  subject: "Exploring backend roles",
  body: "Hi Jane,\n\nI am reaching out…",
  provider: "gemini",
  model: "gemini-2.5-flash",
  status: "draft",
  createdAt: "2026-08-23T10:00:00Z",
  updatedAt: "2026-08-23T10:00:00Z",
};

const GENERATED = {
  ...EXISTING,
  id: 8,
  subject: "Application for Backend Engineer",
  body: "Dear Jane,\n\nI would like to apply…",
};

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
    switch (command) {
      case "generated_email_list":
        return Promise.resolve([EXISTING]);
      case "application_list":
        return Promise.resolve([]);
      case "email_history_list":
        return Promise.resolve([]);
      case "email_template_list":
        return Promise.resolve([]);
      case "ai_generate_email":
        return Promise.resolve(GENERATED);
      case "generated_email_get":
        return Promise.resolve(EXISTING);
      case "get_setting":
        return Promise.resolve(
          args?.key === "compose.default_role" ? "backend/ai intern" : null,
        );
      case "resume_file_list":
        return Promise.resolve([
          {
            id: 5,
            kind: "pdf_master",
            originalFilename: "Best_Resume.pdf",
            storedPath: "/data/resumes/best.pdf",
            sha256: "aaa",
            fileSize: 100,
            notes: "",
            createdAt: "2026-08-20T00:00:00Z",
            updatedAt: "2026-08-20T00:00:00Z",
          },
        ]);
      default:
        return Promise.resolve(null);
    }
  });
});

afterEach(cleanup);

describe("Emails page", () => {
  it("renders compose form and existing drafts", async () => {
    render(<MemoryRouter><Emails /></MemoryRouter>);

    expect(screen.getByText("Compose")).toBeTruthy();
    expect(await screen.findByText(/exploring backend roles/i)).toBeTruthy();
    expect(invokeMock).toHaveBeenCalledWith("generated_email_list", { status: undefined });
  });

  it("generates a draft through IPC and opens it in the editor", async () => {
    const user = userEvent.setup();
    render(<MemoryRouter><Emails /></MemoryRouter>);

    await user.type(await screen.findByLabelText(/recipient email/i), "jane@acme.com");
    await user.click(screen.getByRole("button", { name: /generate draft/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "ai_generate_email",
        expect.objectContaining({
          request: expect.objectContaining({
            recipientEmail: "jane@acme.com",
            emailType: "cold_outreach",
          }),
        }),
      );
    });
    expect(await screen.findByText(/would like to apply/i)).toBeTruthy();
    expect(screen.getAllByText("Draft").length).toBeGreaterThan(0);
  });

  it("loads a history item into the editor on click", async () => {
    const user = userEvent.setup();
    render(<MemoryRouter><Emails /></MemoryRouter>);

    await user.click(await screen.findByText(/exploring backend roles/i));
    expect(await screen.findByDisplayValue(/reaching out/i)).toBeTruthy();
    expect(screen.getByLabelText(/subject/i)).toBeTruthy();
  });

  it("saves edits and flips draft to edited via IPC", async () => {
    const user = userEvent.setup();
    render(<MemoryRouter><Emails /></MemoryRouter>);

    await user.click(await screen.findByText(/exploring backend roles/i));
    const bodyBox = screen.getByLabelText(/^body/i);
    await user.clear(bodyBox);
    await user.type(bodyBox, "Rewritten body");

    const saveButton = screen.getByRole("button", { name: /save changes/i });
    await user.click(saveButton);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("generated_email_update", {
        id: 7,
        subject: EXISTING.subject,
        body: "Rewritten body",
      });
    });
  });
});

describe("Compose defaults", () => {
  it("pre-fills role from saved defaults and saves new ones", async () => {
    const user = userEvent.setup();
    render(<MemoryRouter><Emails /></MemoryRouter>);

    // default role loaded from settings
    expect(await screen.findByDisplayValue(/backend\/ai intern/i)).toBeTruthy();

    // saving defaults persists current compose values
    const roleInput = screen.getByLabelText(/role/i);
    await user.clear(roleInput);
    await user.type(roleInput, "AI intern");
    await user.click(screen.getByRole("button", { name: /save as default/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("set_setting", {
        key: "compose.default_role",
        value: "AI intern",
      });
      expect(invokeMock).toHaveBeenCalledWith("set_setting", {
        key: "compose.default_email_type",
        value: "cold_outreach",
      });
    });
  });
});
