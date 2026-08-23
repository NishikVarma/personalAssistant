import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import Settings from "@/pages/Settings";

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation((command: string) => {
    switch (command) {
      case "get_app_info":
        return Promise.resolve({
          appVersion: "0.1.0",
          dbPath: "/tmp/assistant.db",
          schemaVersion: 1,
        });
      case "ai_get_config":
        return Promise.resolve({ model: "gemini-2.5-flash", hasApiKey: false });
      case "ai_test_connection":
        return Promise.resolve({
          ok: true,
          latencyMs: 420,
          error: null,
          model: "gemini-2.5-flash",
        });
      default:
        return Promise.resolve(null);
    }
  });
});

afterEach(cleanup);

describe("Settings AI provider card", () => {
  it("loads and shows the current AI config", async () => {
    render(<Settings />);

    expect(await screen.findByText("AI provider")).toBeTruthy();
    expect(
      await screen.findByDisplayValue("gemini-2.5-flash"),
    ).toBeTruthy();
    expect(screen.getByPlaceholderText(/Paste API key/i)).toBeTruthy();
    expect(invokeMock).toHaveBeenCalledWith("ai_get_config");
  });

  it("shows the stored-key placeholder once a key exists", async () => {
    invokeMock.mockImplementation((command: string) =>
      command === "ai_get_config"
        ? Promise.resolve({ model: "gemini-2.5-flash", hasApiKey: true })
        : command === "get_app_info"
          ? Promise.resolve({ appVersion: "0.1.0", dbPath: "/x.db", schemaVersion: 1 })
          : Promise.resolve(null),
    );
    render(<Settings />);

    expect(await screen.findByPlaceholderText(/leave blank to keep/i)).toBeTruthy();
    // removal affordance appears only when a key is configured
    await waitFor(() => {
      expect(screen.getByLabelText("Delete")).toBeTruthy();
    });
  });

  it("tests the connection and reports latency", async () => {
    const user = userEvent.setup();
    render(<Settings />);

    const button = await screen.findByRole("button", { name: /test connection/i });
    await user.click(button);

    expect(await screen.findByText(/connected \(420 ms\)/i)).toBeTruthy();
    expect(invokeMock).toHaveBeenCalledWith("ai_test_connection");
  });

  it("saves the model through IPC", async () => {
    const user = userEvent.setup();
    render(<Settings />);

    const input = await screen.findByDisplayValue("gemini-2.5-flash");
    await user.clear(input);
    await user.type(input, "gemini-2.5-pro");
    // the AI card renders before the key-value card, so its Save button is first
    const saveButton = screen.getAllByRole("button", { name: /^save$/i })[0];
    await user.click(saveButton);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("ai_set_model", { model: "gemini-2.5-pro" });
    });
  });
});
