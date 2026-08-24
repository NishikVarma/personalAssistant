import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import HistoryCard from "@/components/emails/HistoryCard";
import TemplatesCard from "@/components/emails/TemplatesCard";

const SENT_ROW = {
  id: 1,
  direction: "outgoing",
  applicationId: null,
  contactId: 5,
  generatedEmailId: 3,
  gmailMessageId: "m1",
  gmailThreadId: "t1",
  emailType: "cold_outreach",
  recipientEmail: "hr@acme.com",
  subject: "Cold outreach to Acme",
  body: "Hi",
  deliveryMethod: "gmail_api",
  status: "sent",
  responseStatus: "awaiting",
  occurredAt: "2026-08-25T10:00:00Z",
  createdAt: "2026-08-25T10:00:00Z",
};

const TEMPLATE = {
  id: 1,
  emailType: "cold_outreach",
  role: "Backend",
  companyOrIndustry: "Acme",
  subjectTemplate: "Hi Acme",
  bodyTemplate: "Hello",
  variablesJson: "[]",
  source: "generated",
  successCount: 0,
  timesUsed: 3,
  lastUsedAt: "2026-08-25T09:00:00Z",
  createdAt: "2026-08-25T08:00:00Z",
  updatedAt: "2026-08-25T08:00:00Z",
};

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation((command: string) => {
    switch (command) {
      case "email_history_list":
        return Promise.resolve([SENT_ROW]);
      case "email_template_list":
        return Promise.resolve([TEMPLATE]);
      case "gmail_sync_replies":
        return Promise.resolve({ checked: 1, repliesFound: 1 });
      default:
        return Promise.resolve(null);
    }
  });
});

afterEach(cleanup);

describe("HistoryCard", () => {
  it("renders sent rows with response status controls", async () => {
    render(<HistoryCard applications={[]} contacts={[]} gmailConnected />);

    expect(await screen.findByText(/cold outreach to acme/i)).toBeTruthy();
    expect(screen.getByText("hr@acme.com")).toBeTruthy();
    const select = screen.getByDisplayValue("Awaiting") as HTMLSelectElement;
    expect(select).toBeTruthy();
  });

  it("syncs replies through IPC and reloads history", async () => {
    const user = userEvent.setup();
    render(<HistoryCard applications={[]} contacts={[]} gmailConnected />);

    await user.click(await screen.findByRole("button", { name: /sync replies/i }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("gmail_sync_replies");
    });
    // history is re-fetched after the sync
    await waitFor(() => {
      const listCalls = invokeMock.mock.calls.filter((c) => c[0] === "email_history_list");
      expect(listCalls.length).toBeGreaterThanOrEqual(2);
    });
  });

  it("updates response status through IPC", async () => {
    invokeMock.mockImplementation((command: string) =>
      command === "email_history_list"
        ? Promise.resolve([{ ...SENT_ROW, responseStatus: "replied" }])
        : command === "email_history_set_response"
          ? Promise.resolve({ ...SENT_ROW, responseStatus: "replied" })
          : Promise.resolve(null),
    );
    const user = userEvent.setup();
    render(<HistoryCard applications={[]} contacts={[]} gmailConnected />);

    const select = await screen.findByDisplayValue("Replied");
    await user.selectOptions(select, "no_reply_needed");
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("email_history_set_response", {
        id: 1,
        status: "no_reply_needed",
      });
    });
  });
});

describe("TemplatesCard", () => {
  it("lists templates with usage info", async () => {
    render(<TemplatesCard />);

    expect(await screen.findByText(/hi acme/i)).toBeTruthy();
    expect(screen.getByText(/used 3×/i)).toBeTruthy();
    expect(screen.getByText(/backend at acme/i)).toBeTruthy();
  });
});
