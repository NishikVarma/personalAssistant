import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(true),
  requestPermission: vi.fn().mockResolvedValue("granted"),
  sendNotification: vi.fn(),
}));

import FollowUps from "@/pages/FollowUps";

const DUE = {
  id: 1,
  applicationId: 10,
  contactId: 20,
  originatingEmailId: 30,
  sequence: 1,
  scheduledFor: "2026-08-20T12:00:00+00:00",
  status: "due",
  suppressedReason: null,
  completedAt: null,
  createdAt: "2026-08-13T12:00:00+00:00",
  updatedAt: "2026-08-25T12:00:00+00:00",
};

const APPS = [{ id: 10, company: "Acme", role: "Backend", status: "applied" }];
const CONTACTS = [{ id: 20, name: "Jane", email: "jane@acme.com", color: null }];

function renderPage() {
  return render(
    <MemoryRouter>
      <FollowUps />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation((command: string) => {
    switch (command) {
      case "follow_up_config_get":
        return Promise.resolve({ days: 7, secondDays: null, autoSchedule: true });
      case "follow_up_due":
        return Promise.resolve([DUE]);
      case "follow_up_list":
        return Promise.resolve([]);
      case "application_list":
        return Promise.resolve(APPS);
      case "contact_list":
        return Promise.resolve(CONTACTS);
      case "follow_up_due_count":
        return Promise.resolve(1);
      default:
        return Promise.resolve(null);
    }
  });
});

afterEach(cleanup);

describe("FollowUps page", () => {
  it("renders cadence config loaded from settings", async () => {
    renderPage();

    expect(await screen.findByDisplayValue("7")).toBeTruthy();
    expect(screen.getByRole("button", { name: /auto-schedule/i }).textContent).toContain("Enabled");
    expect(invokeMock).toHaveBeenCalledWith("follow_up_config_get");
  });

  it("lists due follow-ups with application and contact context", async () => {
    renderPage();

    expect(await screen.findByText(/acme · backend/i)).toBeTruthy();
    expect(screen.getByText(/jane/i)).toBeTruthy();
    expect(screen.getByText(/Due · #1/i)).toBeTruthy();
  });

  it("drafts a follow-up through IPC", async () => {
    const user = userEvent.setup();
    invokeMock.mockImplementation((command: string) => {
      switch (command) {
        case "follow_up_config_get":
          return Promise.resolve({ days: 7, secondDays: null, autoSchedule: true });
        case "follow_up_due":
          return Promise.resolve([DUE]);
        case "follow_up_list":
          return Promise.resolve([]);
        case "application_list":
          return Promise.resolve(APPS);
        case "contact_list":
          return Promise.resolve(CONTACTS);
        case "follow_up_draft":
          return Promise.resolve({
            id: 99,
            applicationId: 10,
            contactId: 20,
            emailType: "follow_up",
            recipientEmail: "jane@acme.com",
            recipientName: "Jane",
            subject: "Re: Hello Acme",
            body: "Following up…",
            provider: "gemini",
            model: "gemini-2.5-flash",
            status: "draft",
            followUpId: 1,
            createdAt: "2026-08-26T10:00:00Z",
            updatedAt: "2026-08-26T10:00:00Z",
          });
        default:
          return Promise.resolve(null);
      }
    });
    renderPage();

    await user.click(await screen.findByRole("button", { name: /draft follow-up/i }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("follow_up_draft", { id: 1 });
    });
  });

  it("cancels a follow-up after confirmation", async () => {
    const user = userEvent.setup();
    renderPage();

    await screen.findByText(/acme · backend/i);
    await user.click(screen.getByLabelText("Delete"));
    await user.click(await screen.findByRole("button", { name: /^cancel$/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("follow_up_cancel", { id: 1 });
    });
  });
});
