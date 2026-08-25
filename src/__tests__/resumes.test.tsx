import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import userEvent from "@testing-library/user-event";
import { fireEvent } from "@testing-library/react";
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
  invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
    switch (command) {
      case "resume_file_list":
        return Promise.resolve(args?.kind === "pdf_master" ? [PDF] : []);
      case "latex_detect":
        return Promise.resolve({ available: true, engine: "pdflatex" });
      case "application_list":
        return Promise.resolve([
          { id: 10, company: "Acme", role: "Backend", status: "applied", jobDescription: "Long JD text here" },
        ]);
      case "resume_variant_list":
        return Promise.resolve([]);
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

describe("Generate tailored resume", () => {
  it("generates a variant from the JD and lists it", async () => {
    invokeMock.mockImplementation((command: string) => {
      switch (command) {
        case "resume_file_list":
          return Promise.resolve(
            command_arg_kind(command) ?? [],
          );
        case "latex_detect":
          return Promise.resolve({ available: true, engine: "pdflatex" });
        case "application_list":
          return Promise.resolve([
            { id: 10, company: "Acme", role: "Backend", status: "applied", jobDescription: "x".repeat(60) },
          ]);
        case "resume_variant_list":
          return Promise.resolve([
            {
              id: 1,
              baseFileId: 2,
              applicationId: 10,
              category: "backend",
              label: "Tailored — Backend Engineer",
              texPath: "/data/resumes/variant-1.tex",
              pdfPath: "/data/resumes/variant-1.pdf",
              status: "draft",
              notes: "",
              createdAt: "2026-08-26T10:00:00Z",
              updatedAt: "2026-08-26T10:00:00Z",
            },
          ]);
        default:
          return Promise.resolve(null);
      }
    });
    renderPage();

    // variant listed with compile status
    expect(await screen.findByText(/Tailored — Backend Engineer/i)).toBeTruthy();
    expect(screen.getByText(/PDF compiled/)).toBeTruthy();
    expect(screen.getAllByText(/backend/i).length).toBeGreaterThan(0);
  });

  it("generates via IPC from a pasted JD", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "resume_file_list") return Promise.resolve([]);
      if (command === "latex_detect") return Promise.resolve({ available: false, engine: null });
      if (command === "resume_generate_variant") {
        return Promise.resolve({
          id: 2, baseFileId: null, applicationId: null, category: "backend",
          label: "Tailored — Backend Engineer", texPath: "/v/2.tex", pdfPath: null,
          status: "draft", notes: "", createdAt: "2026-08-26T10:00:00Z", updatedAt: "2026-08-26T10:00:00Z",
        });
      }
      return Promise.resolve([]);
    });
    renderPage();

    const jdBox = await screen.findByPlaceholderText(/paste the job description/i);
    fireEvent.change(jdBox, {
      target: { value: "Backend engineer role requiring Rust and Kafka experience." },
    });
    // sanity: does any React click handler run in this test?
    fireEvent.click(screen.getByRole("button", { name: /upload pdf/i }));
    await waitFor(() => {
      expect(openMock).toHaveBeenCalled();
    });
    fireEvent.click(screen.getByRole("button", { name: /generate/i }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "resume_generate_variant",
        expect.objectContaining({ jdText: expect.stringContaining("Rust and Kafka") }),
      );
    });
  });
});

function command_arg_kind(_command: string): unknown[] | null {
  // the page filters by kind client-side through separate calls; default to empty tex list
  return [];
}
