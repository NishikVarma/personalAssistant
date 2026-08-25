import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
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

import Bulk from "@/pages/Bulk";

const PREVIEW = {
  headers: ["name", "email", "company"],
  sampleRows: [["Jane Doe", "jane@acme.com", "Acme"]],
  totalDataRows: 2,
};

const ROW_STATUSES = [
  {
    rowIndex: 0,
    name: "Jane Doe",
    email: "jane@acme.com",
    company: "Acme",
    role: "",
    status: "ready",
    detail: null,
    generatedEmailId: 11,
  },
  {
    rowIndex: 1,
    name: "Bad Row",
    email: "not-an-email",
    company: "",
    role: "",
    status: "invalid",
    detail: "invalid or missing email",
    generatedEmailId: null,
  },
];

const SENT_DRAFT = {
  id: 11,
  applicationId: null,
  contactId: null,
  emailType: "cold_outreach",
  recipientEmail: "jane@acme.com",
  recipientName: "Jane Doe",
  subject: "Hi Jane",
  body: "Hello",
  provider: "gemini",
  model: "gemini-2.5-flash",
  status: "sent",
  followUpId: null,
  bulkBatchId: 1,
  createdAt: "2026-08-26T10:00:00Z",
  updatedAt: "2026-08-26T10:00:00Z",
};

function renderPage() {
  return render(
    <MemoryRouter>
      <Bulk />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  invokeMock.mockReset();
  openMock.mockReset();
  invokeMock.mockImplementation((command: string) => {
    switch (command) {
      case "bulk_import_preview":
        return Promise.resolve(PREVIEW);
      case "bulk_batch_create":
        return Promise.resolve({
          id: 1,
          emailType: "cold_outreach",
          applicationId: null,
          status: "draft",
          totalCount: 0,
          sentCount: 0,
          failedCount: 0,
          createdAt: "2026-08-26T10:00:00Z",
          updatedAt: "2026-08-26T10:00:00Z",
        });
      case "bulk_generate":
        return Promise.resolve(ROW_STATUSES);
      case "generated_email_get":
        return Promise.resolve(SENT_DRAFT);
      case "generated_email_set_status":
        return Promise.resolve({ ...SENT_DRAFT, status: "approved" });
      case "email_send":
        return Promise.resolve({ ...SENT_DRAFT });
      case "bulk_batch_finish":
        return Promise.resolve({ id: 1, status: "sent" });
      case "application_list":
        return Promise.resolve([]);
      default:
        return Promise.resolve(null);
    }
  });
});

afterEach(cleanup);

describe("Bulk outreach", () => {
  it("imports a file and shows the preview with guessed mappings", async () => {
    openMock.mockResolvedValue("/data/contacts.csv");
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: /choose file/i }));

    expect(await screen.findByText("2")).toBeTruthy(); // data row count
    expect(screen.getByText(/data rows/)).toBeTruthy();
    // email column auto-guessed from the header
    const emailSelect = screen
      .getAllByRole("combobox")
      .find((s) => (s as HTMLSelectElement).value === "email");
    expect(emailSelect).toBeTruthy();
    expect(invokeMock).toHaveBeenCalledWith("bulk_import_preview", {
      sourcePath: "/data/contacts.csv",
    });
  });

  it("generates drafts and shows per-recipient status chips", async () => {
    openMock.mockResolvedValue("/data/contacts.csv");
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: /choose file/i }));
    await waitFor(() => {
      expect(screen.getAllByRole("combobox").length).toBeGreaterThan(3);
    });
    await user.click(screen.getByRole("button", { name: /create batch & generate/i }));

    expect(await screen.findByText(/1 ready/i)).toBeTruthy();
    expect(screen.getAllByText("Jane Doe").length).toBeGreaterThan(0);
    expect(screen.getByText("Invalid email")).toBeTruthy();
    expect(invokeMock).toHaveBeenCalledWith(
      "bulk_generate",
      expect.objectContaining({ batchId: 1 }),
    );
  });

  it("previews a generated draft", async () => {
    openMock.mockResolvedValue("/data/contacts.csv");
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: /choose file/i }));
    await user.click(await screen.findByRole("button", { name: /create batch & generate/i }));
    await user.click(await screen.findByRole("button", { name: /preview/i }));

    expect(await screen.findByText("Hi Jane")).toBeTruthy();
    expect(invokeMock).toHaveBeenCalledWith("generated_email_get", { id: 11 });
  });

  it("sends ready drafts sequentially after confirmation", async () => {
    openMock.mockResolvedValue("/data/contacts.csv");
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: /choose file/i }));
    await user.click(await screen.findByRole("button", { name: /create batch & generate/i }));
    await user.click(await screen.findByRole("button", { name: /send 1 email/i }));

    const dialog = await screen.findByRole("dialog");
    expect(dialog.textContent).toContain("Send 1 emails?");
    await user.click(screen.getByRole("button", { name: /confirm & send/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("generated_email_set_status", {
        id: 11,
        status: "approved",
      });
      expect(invokeMock).toHaveBeenCalledWith("email_send", {
        id: 11,
        attachmentPath: null,
        force: false,
      });
      expect(invokeMock).toHaveBeenCalledWith("bulk_batch_finish", { id: 1, status: "sent" });
    });
  });
});
