import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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

describe("FormDialog opt-in month/year dates", () => {
  const fields = [
    { name: "startDate", label: "Start date", type: "date" as const },
    { name: "grade", label: "Grade", type: "text" as const },
  ];

  it("hides the date picker behind an Add button when empty", () => {
    render(
      <FormDialog
        title="Test"
        fields={fields}
        initial={{}}
        onSubmit={async () => {}}
        onClose={() => {}}
      />,
    );

    expect(screen.getByRole("button", { name: /add start date/i })).toBeTruthy();
    expect(screen.queryByLabelText(/start date month/i)).toBeNull();
  });

  it("reveals month and year selects when the Add button is clicked", async () => {
    const user = userEvent.setup();
    render(
      <FormDialog
        title="Test"
        fields={fields}
        initial={{}}
        onSubmit={async () => {}}
        onClose={() => {}}
      />,
    );

    await user.click(screen.getByRole("button", { name: /add start date/i }));
    expect(screen.getByLabelText(/start date month/i)).toBeTruthy();
    expect(screen.getByLabelText(/start date year/i)).toBeTruthy();
  });

  it("submits YYYY-MM once both month and year are chosen", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(
      <FormDialog
        title="Test"
        fields={fields}
        initial={{}}
        onSubmit={onSubmit}
        onClose={() => {}}
      />,
    );

    await user.click(screen.getByRole("button", { name: /add start date/i }));
    await user.selectOptions(screen.getByLabelText(/start date month/i), "08");
    await user.selectOptions(screen.getByLabelText(/start date year/i), "2022");
    await user.click(screen.getByRole("button", { name: /save/i }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({ startDate: "2022-08" }));
    });
  });

  it("submits an empty value when revealed but incomplete", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(
      <FormDialog
        title="Test"
        fields={fields}
        initial={{}}
        onSubmit={onSubmit}
        onClose={() => {}}
      />,
    );

    await user.click(screen.getByRole("button", { name: /add start date/i }));
    await user.selectOptions(screen.getByLabelText(/start date year/i), "2022");
    await user.click(screen.getByRole("button", { name: /save/i }));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({ startDate: "" }));
    });
  });

  it("removing the date returns to the Add button and clears the value", async () => {
    const user = userEvent.setup();
    render(
      <FormDialog
        title="Test"
        fields={fields}
        initial={{ startDate: "2022-08-01" }}
        onSubmit={async () => {}}
        onClose={() => {}}
      />,
    );

    // edit mode with an existing date reveals the picker pre-filled
    const month = screen.getByLabelText(/start date month/i) as HTMLSelectElement;
    const year = screen.getByLabelText(/start date year/i) as HTMLSelectElement;
    expect(month.value).toBe("08");
    expect(year.value).toBe("2022");

    await user.click(screen.getByRole("button", { name: /remove start date/i }));
    expect(screen.getByRole("button", { name: /add start date/i })).toBeTruthy();
    expect(screen.queryByLabelText(/start date month/i)).toBeNull();
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

    // switching to a known kind swaps in the optional placeholder
    fireEvent.change(screen.getByLabelText(/kind/i), { target: { value: "github" } });
    expect(labelInput.placeholder).toContain("Optional");
  });

  it("marks label required only for Other", async () => {
    render(<LinksSection />);

    const addButtons = await screen.findAllByRole("button", { name: /^add$/i });
    fireEvent.click(addButtons[addButtons.length - 1]);
    await screen.findByRole("dialog");

    // other (default): asterisk present
    const labelLabel = screen.getByLabelText(/label/i).closest("div")?.querySelector("label");
    expect(labelLabel?.textContent).toContain("*");

    fireEvent.change(screen.getByLabelText(/kind/i), { target: { value: "github" } });
    expect(labelLabel?.textContent).not.toContain("*");
  });
});
