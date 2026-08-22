import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  appVersion: string;
  dbPath: string;
  schemaVersion: number;
}

export const ipc = {
  appInfo: () => invoke<AppInfo>("get_app_info"),
  getSetting: (key: string) => invoke<string | null>("get_setting", { key }),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
  deleteSetting: (key: string) => invoke<boolean>("delete_setting", { key }),
};
