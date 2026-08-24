import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";

const DEDUP_KEY = "follow_up_notify";

/** Fires one OS notification for due follow-ups, at most once per day per count. */
export async function notifyDueFollowUps(count: number): Promise<void> {
  if (count <= 0) return;
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === "granted";
    }
    if (!granted) return;

    const today = new Date().toISOString().slice(0, 10);
    const last = localStorage.getItem(DEDUP_KEY);
    if (last === `${today}:${count}`) return; // already told you about this today
    localStorage.setItem(DEDUP_KEY, `${today}:${count}`);

    sendNotification({
      title: "Follow-ups due",
      body: `You have ${count} follow-up${count === 1 ? "" : "s"} due.`,
    });
  } catch {
    // notifications are best-effort; never break the page over them
  }
}
