import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import FormDialog from "@/components/profile/FormDialog";
import LinksSection from "@/components/profile/sections/LinksSection";

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation((command: string) =>
    command === "link_list" ? Promise.resolve([]) : Promise.resolve(null),
  );
});

afterEach(cleanup);

describe("FormDialog date fields", () => {
  const fields = [
    { name: "startDate", label: "Start date", type: "date" as const },
    { name: "grade", label: "Grade", type: "text" as const },
  ];

  it("advances focus when a complete date is picked", () => {
    render(
      <FormDialog
        title="Test"
        fields={fields}
        initial={{}}
        onSubmit={async () => {}}
        onClose={() => {}}
      />,
    );

    const dateInput = screen.getByLabelText(/start date/i);
    fireEvent.change(dateInput, { target: { value: "2026-01-15" } });

    expect(document.activeElement).toBe(screen.getByLabelText(/grade/i));
  });

  it("does not advance for incomplete dates", () => {
    render(
      <FormDialog
        title="Test"
        fields={fields}
        initial={{}}
        onSubmit={async () => {}}
        onClose={() => {}}
      />,
    );

    const dateInput = screen.getByLabelText(/start date/i);
    fireEvent.change(dateInput, { target: { value: "2026-01" } });

    expect(document.activeElement).toBe(dateInput);
  });

  it("advances focus on Enter", () => {
    render(
      <FormDialog
        title="Test"
        fields={fields}
        initial={{}}
        onSubmit={async () => {}}
        onClose={() => {}}
      />,
    );

    const dateInput = screen.getByLabelText(/start date/i);
    fireEvent.keyDown(dateInput, { key: "Enter" });

    expect(document.activeElement).toBe(screen.getByLabelText(/grade/i));
  });

  it("marks non-required dates as optional", () => {
    render(
      <FormDialog
        title="Test"
        fields={fields}
        initial={{}}
        onSubmit={async () => {}}
        onClose={() => {}}
      />,
    );

    expect(screen.getByText("(optional)")).toBeTruthy();
  });
});

describe("Links dialog", () => {
  it("shows the kind dropdown first and the custom placeholder for Other", async () => {
    render(<LinksSection />);

    const addButtons = await screen.findAllByRole("button", { name: /^add$/i });
    await waitFor(() => expect(addButtons.length).toBeGreaterThan(0));
    fireEvent.click(addButtons[addButtons.length - 1]);

    const dialog = await screen.findByRole("dialog");
    const firstLabel = dialog.querySelector("label")?.textContent;
    expect(firstLabel).toContain("Kind");

    // add mode defaults to kind=other, so the custom placeholder shows immediately
    const labelInput = screen.getByLabelText(/label/i) as HTMLInputElement;
    expect(labelInput.placeholder).toContain("Blog");

    // switching to a known kind clears the custom placeholder
    fireEvent.change(screen.getByLabelText(/kind/i), { target: { value: "github" } });
    expect(labelInput.placeholder).not.toContain("Blog");

    // and back to other brings it back
    fireEvent.change(screen.getByLabelText(/kind/i), { target: { value: "other" } });
    expect(labelInput.placeholder).toContain("Blog");
  });
});
