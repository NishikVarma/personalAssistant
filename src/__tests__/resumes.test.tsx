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

import Resumes from "@/pages/Resumes";

const PDF = {
  id: 1,
  kind: "pdf_master",
  originalFilename: "Nishik_Backend.pdf",
  storedPath: "/data/resumes/abc.pdf",
  sha256: "abcdef1234567890",
  fileSize: 204800,
  notes: "",
  createdAt: "2026-08-26T10:00:00Z",
  updatedAt: "2026-08-26T10:00:00Z",
};

function renderPage() {
  return render(
    <MemoryRouter>
      <Resumes />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  invokeMock.mockReset();
  openMock.mockReset();
  invokeMock.mockImplementation((command: string) => {
    switch (command) {
      case "resume_file_list":
        return Promise.resolve(args?.kind === "pdf_master" ? [PDF] : []);
      case "latex_detect":
        return Promise.resolve({ available: true, engine: "pdflatex" });
      default:
        return Promise.resolve(null);
    }
  });
});

afterEach(cleanup);

describe("Resumes page", () => {
  it("lists stored resumes and the LaTeX availability", async () => {
    renderPage();

    expect(await screen.findByText("Nishik_Backend.pdf")).toBeTruthy();
    expect(screen.getByText(/200\.0 KB/)).toBeTruthy();
    expect(screen.getByText(/LaTeX ready · pdflatex/)).toBeTruthy();
    expect(screen.getAllByText(/sha256 abcdef123456/).length).toBeGreaterThan(0);
  });

  it("uploads a pdf through the file dialog", async () => {
    openMock.mockResolvedValue("/home/user/New_Resume.pdf");
    invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
      if (command === "resume_file_upload") {
        return Promise.resolve({ ...PDF, id: 2, originalFilename: "New_Resume.pdf" });
      }
      if (command === "resume_file_list") {
        return Promise.resolve(
          args?.kind === "pdf_master" ? [PDF, { ...PDF, id: 2, originalFilename: "New_Resume.pdf" }] : [],
        );
      }
      if (command === "latex_detect") {
        return Promise.resolve({ available: false, engine: null });
      }
      return Promise.resolve(null);
    });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: /upload pdf/i }));
    await waitFor(() => {
      expect(openMock).toHaveBeenCalledWith(
        expect.objectContaining({ filters: [{ name: "Resume PDF", extensions: ["pdf"] }] }),
      );
      expect(invokeMock).toHaveBeenCalledWith("resume_file_upload", {
        kind: "pdf_master",
        sourcePath: "/home/user/New_Resume.pdf",
      });
    });
  });

  it("shows the no-LaTeX fallback message", async () => {
    invokeMock.mockImplementation((command: string) =>
      command === "resume_file_list"
        ? Promise.resolve([])
        : command === "latex_detect"
          ? Promise.resolve({ available: false, engine: null })
          : Promise.resolve(null),
    );
    renderPage();

    expect(await screen.findByText(/No LaTeX engine found/)).toBeTruthy();
    expect(screen.getByText(/Install TeX Live/i)).toBeTruthy();
  });
});
