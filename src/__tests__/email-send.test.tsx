import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
const { openMock } = vi.hoisted(() => ({ openMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: openMock,
}));

import Emails from "@/pages/Emails";

const APPROVED = {
  id: 9,
  applicationId: null,
  contactId: null,
  emailType: "job_application",
  recipientEmail: "hr@acme.com",
  recipientName: "Jane",
  subject: "Application for Backend role",
  body: "Dear Jane,\n\nI would like to apply…",
  provider: "gemini",
  model: "gemini-2.5-flash",
  status: "approved",
  createdAt: "2026-08-24T10:00:00Z",
  updatedAt: "2026-08-24T10:00:00Z",
};

const SENT = { ...APPROVED, status: "sent" };

beforeEach(() => {
  invokeMock.mockReset();
  openMock.mockReset();
  invokeMock.mockImplementation((command: string) => {
    switch (command) {
      case "generated_email_list":
        return Promise.resolve([APPROVED]);
      case "application_list":
        return Promise.resolve([]);
      case "email_history_list":
        return Promise.resolve([]);
      case "email_template_list":
        return Promise.resolve([]);
      case "google_status":
        return Promise.resolve({ connected: true, accountEmail: "me@gmail.com" });
      case "email_send":
        return Promise.resolve(SENT);
      case "generated_email_update":
        return Promise.resolve(APPROVED);
      default:
        return Promise.resolve(null);
    }
  });
});

afterEach(cleanup);

describe("Emails send flow", () => {
  it("shows Send via Gmail for approved drafts when connected", async () => {
    render(<Emails />);

    await userEvent.setup().click(await screen.findByText(/application for backend role/i));
    expect(await screen.findByRole("button", { name: /send via gmail/i })).toBeTruthy();
  });

  it("hides the send button when Gmail is not connected", async () => {
    invokeMock.mockImplementation((command: string) =>
      command === "google_status"
        ? Promise.resolve({ connected: false, accountEmail: null })
        : command === "generated_email_list"
          ? Promise.resolve([APPROVED])
          : command === "application_list"
            ? Promise.resolve([])
            : Promise.resolve(null),
    );
    render(<Emails />);

    await userEvent.setup().click(await screen.findByText(/application for backend role/i));
    const button = await screen.findByRole("button", { name: /send via gmail/i });
    expect((button as HTMLButtonElement).disabled).toBe(true);
  });

  it("sends through the confirmation dialog", async () => {
    const user = userEvent.setup();
    render(<Emails />);

    await user.click(await screen.findByText(/application for backend role/i));
    await user.click(await screen.findByRole("button", { name: /send via gmail/i }));

    const dialog = await screen.findByRole("dialog");
    expect(dialog.textContent).toContain("hr@acme.com");
    expect(dialog.textContent).toContain("I would like to apply");

    await user.click(screen.getByRole("button", { name: /^send$/i }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("email_send", {
        id: 9,
        attachmentPath: null,
        force: false,
      });
    });
    // status flips to sent: badge appears and the send button disappears
    expect(await screen.findByText("Sent")).toBeTruthy();
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: /send via gmail/i })).toBeNull();
    });
  });

  it("offers a force override when recent outreach is detected", async () => {
    invokeMock.mockImplementation((command: string) => {
      switch (command) {
        case "generated_email_list":
          return Promise.resolve([APPROVED]);
        case "application_list":
          return Promise.resolve([]);
        case "email_history_list":
          return Promise.resolve([]);
        case "email_template_list":
          return Promise.resolve([]);
        case "google_status":
          return Promise.resolve({ connected: true, accountEmail: "me@gmail.com" });
        case "email_send":
          return Promise.reject(
            new Error("recent outreach: an email was already sent to hr@acme.com"),
          );
        default:
          return Promise.resolve(null);
      }
    });
    const user = userEvent.setup();
    render(<Emails />);

    await user.click(await screen.findByText(/application for backend role/i));
    await user.click(await screen.findByRole("button", { name: /send via gmail/i }));
    await user.click(await screen.findByRole("button", { name: /^send$/i }));

    const warning = await screen.findByText(/recent outreach/i);
    expect(warning).toBeTruthy();

    // send stays disabled until the override is checked
    const sendButton = screen.getByRole("button", { name: /^send$/i }) as HTMLButtonElement;
    expect(sendButton.disabled).toBe(true);
    await user.click(screen.getByLabelText(/send anyway/i));
    expect(sendButton.disabled).toBe(false);
  });
});
