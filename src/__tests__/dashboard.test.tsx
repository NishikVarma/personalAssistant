import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import Dashboard from "@/pages/Dashboard";

const APPS = [
  { id: 1, company: "Acme", role: "BE", status: "applied", updatedAt: "2026-08-23T10:00:00Z" },
  { id: 2, company: "Globex", role: "FS", status: "interview", updatedAt: "2026-08-22T10:00:00Z" },
  { id: 3, company: "Initech", role: "DE", status: "saved", updatedAt: "2026-08-21T10:00:00Z" },
];

const EMAILS = [
  { id: 1, subject: "Cold outreach to Acme", status: "approved", createdAt: "2026-08-23T09:00:00Z" },
  { id: 2, subject: "Follow-up with Globex", status: "draft", createdAt: "2026-08-23T08:00:00Z" },
];

function renderDashboard() {
  return render(
    <MemoryRouter>
      <Dashboard />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation((command: string) => {
    switch (command) {
      case "application_list":
        return Promise.resolve(APPS);
      case "contact_list":
        return Promise.resolve([{ id: 1 }, { id: 2 }]);
      case "generated_email_list":
        return Promise.resolve(EMAILS);
      case "get_app_info":
        return Promise.resolve({
          appVersion: "0.1.0",
          dbPath: "/tmp/assistant.db",
          schemaVersion: 1,
        });
      default:
        return Promise.resolve(null);
    }
  });
});

afterEach(cleanup);

describe("Dashboard", () => {
  it("renders live stats from IPC data", async () => {
    renderDashboard();

    const appsCard = await screen.findByTitle("Total tracked");
    await waitFor(() => expect(appsCard.textContent).toContain("3"));
    expect(screen.getByTitle("Recruiters & referrals").textContent).toContain("2");
    expect(screen.getByTitle("Not yet approved").textContent).toContain("1");
    expect(screen.getByTitle("Waiting to send").textContent).toContain("1");
  });

  it("shows pipeline stages and recent activity once loaded", async () => {
    renderDashboard();

    // wait until the underlying lists resolved
    await screen.findByText(/cold outreach to acme/i);

    for (const stage of ["Backlog", "Outreach", "Interviews", "Closed"]) {
      expect(screen.getByText(stage)).toBeTruthy();
    }

    const recentCard = screen
      .getByText("Recent applications")
      .closest("[data-slot=card]") as HTMLElement;
    await waitFor(() => expect(within(recentCard).getByText(/^acme/i)).toBeTruthy());
    expect(within(recentCard).getByText(/globex/i)).toBeTruthy();

    const draftsCard = screen
      .getByText("Recent drafts")
      .closest("[data-slot=card]") as HTMLElement;
    expect(within(draftsCard).getByText(/cold outreach to acme/i)).toBeTruthy();
    expect(within(draftsCard).getByText(/follow-up with globex/i)).toBeTruthy();
  });

  it("surfaces backend errors instead of a blank page", async () => {
    invokeMock.mockImplementation(() => Promise.reject(new Error("boom")));
    renderDashboard();

    expect(await screen.findByText(/backend error/i)).toBeTruthy();
  });
});
