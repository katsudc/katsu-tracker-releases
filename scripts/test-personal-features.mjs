import test from "node:test";
import assert from "node:assert/strict";
import { parseSettingsFile, settingsFile, SETTINGS_FILE_LIMIT } from "../src/features/preferenceData.ts";
import { cleanLibrary, parseTime, playbackUrl, validLineup, videoUrl } from "../src/features/libraryData.ts";
import { dateValue, localDate, sessionBounds, sessionMatches } from "../src/features/sessionData.ts";
import { diagnosticChecks } from "../src/features/diagnosticData.ts";
import { featureMessages } from "../src/features/messages.ts";

function preferences() {
  return {
    theme: "rose", locale: "tr", hotkey: "Ctrl+F2", windowStyle: "free",
    autoLockDelaySeconds: 3, discordRpcEnabled: true, minimizeToTray: false,
    autoLockAgent: "11111111-1111-4111-8111-111111111111", pausedAutoLockAgent: null,
    mapAgentPreferences: { Haven: "22222222-2222-4222-8222-222222222222" }, spikeEnabled: true,
    appearance: { position: { x: 0.2, y: 0.8 }, font_size: 40, background_opacity: 0.5, game_bar: { font_size: 62, background_opacity: 0.1 } },
    stream: { enabled: false, hideNames: true, quietNotifications: true, hideMusic: true },
  };
}
const serialized = p => JSON.stringify({ format: "katsu-tracker-settings", version: 1, createdAt: "2026-10-29T10:00:00Z", preferences: p });
const lineup = () => ({
  id: "sample-1", title: "A long — kolay atış", notes: "Köşeye yaslan.\nİkinci çizgiyi hizala.",
  agent: "brimstone", map: "haven", side: "attack", url: "https://www.youtube.com/watch?v=RRdK1cVjZzk",
  seconds: 85, favorite: true, createdAt: Date.now(),
});
test("portable settings preserve independent renderer appearance and paused agent state", () => {
  const p = preferences(); p.pausedAutoLockAgent = p.autoLockAgent; p.autoLockAgent = null;
  assert.deepEqual(settingsFile(p).preferences, { ...p, closeToTray: false, startMinimized: false });
  assert.deepEqual(parseSettingsFile("\uFEFF" + serialized(p)).preferences, { ...p, closeToTray: false, startMinimized: false });
});
test("unrecognized fields and credentials cannot enter exported/imported stores", () => {
  const p = preferences();
  p.accessToken = "fixture-private-token"; p.profile = { name: "private#123" }; p.stream.token = "another-private";
  const file = settingsFile(p);
  assert(!JSON.stringify(file).includes("private"));
  assert.deepEqual(Object.keys(file.preferences.stream).sort(), ["enabled", "hideMusic", "hideNames", "quietNotifications"]);
});
test("invalid transfer versions and oversized files are rejected before application", () => {
  assert.throws(() => parseSettingsFile(serialized(preferences()).replace('"version":1', '"version":2')));
  assert.throws(() => parseSettingsFile(" ".repeat(SETTINGS_FILE_LIMIT + 1)));
  assert.throws(() => parseSettingsFile('{"format":"katsu-tracker-settings"}'));
});
test("hostile and out-of-range transfer values cannot reach native commands", () => {
  for (const mutate of [
    p => p.appearance.font_size = 1000, p => p.appearance.position.x = -1,
    p => p.appearance.game_bar.background_opacity = 2, p => p.hotkey = "F2;execute",
    p => p.autoLockDelaySeconds = 1.3, p => p.stream.enabled = "true",
    p => p.mapAgentPreferences = JSON.parse('{"__proto__":"agent"}'),
  ]) { const p = preferences(); mutate(p); assert.throws(() => parseSettingsFile(serialized(p))); }
});
test("video links require HTTPS and exact supported hosts", () => {
  const credentialFixture = new URL(lineup().url); credentialFixture.username = "fixture"; credentialFixture.password = "test-only";
  for (const url of [
    "javascript:alert(1)", "file:///C:/Windows/system.ini", "https://youtube.com.attacker.test/watch?v=RRdK1cVjZzk",
    credentialFixture.toString(), "https://tracker.gg.evil.test/valorant/guides/clips",
    "https://www.youtube.com/redirect?q=evil", "https://tracker.gg/not-a-guide",
  ]) assert.equal(videoUrl(url), null, url);
  assert(videoUrl(lineup().url));
  assert(videoUrl("https://tracker.gg/valorant/guides/clips/example"));
});
test("timestamps are validated and YouTube opens at the saved time", () => {
  assert.equal(parseTime("1:25"), 85); assert.equal(parseTime(""), 0);
  for (const time of ["1:99", "-1:00", "foo", "9999:59"]) assert.equal(parseTime(time), null);
  assert.equal(new URL(playbackUrl(lineup())).searchParams.get("t"), "85s");
  const item = { ...lineup(), url: "https://tracker.gg/valorant/guides/clips/example" };
  assert.equal(playbackUrl(item), item.url);
});
test("library persistence discards malformed records and duplicate IDs", () => {
  assert(validLineup(lineup()));
  const rows = cleanLibrary([lineup(), { ...lineup(), notes: "duplicate" }, { ...lineup(), id: "bad", url: "file:///private" }, null]);
  assert.equal(rows.length, 1); assert.equal(rows[0].notes, lineup().notes);
  assert.equal(cleanLibrary(Array.from({ length: 300 }, (_, i) => ({ ...lineup(), id: "sample-" + i }))).length, 200);
});
test("session date ranges use local calendar days and an exclusive end boundary", () => {
  const today = new Date(2026, 9, 29, 12).getTime();
  const bounds = sessionBounds("today", today, null, "", "");
  assert.deepEqual(bounds, [new Date(2026, 9, 29).getTime(), new Date(2026, 9, 30).getTime()]);
  const rows = [{ started_at: bounds[0] - 1 }, { started_at: bounds[0] }, { started_at: bounds[1] - 1 }, { started_at: bounds[1] }, { started_at: null }];
  assert.equal(sessionMatches(rows, bounds).length, 2);
  assert.equal(localDate(new Date(bounds[0])), "2026-10-29");
});
test("custom dates reject rollovers and reversed ranges", () => {
  assert.equal(dateValue("2026-02-30"), null);
  assert.equal(sessionBounds("custom", Date.now(), null, "2026-10-30", "2026-10-29"), null);
  assert.equal(sessionMatches([], null).length, 0);
});
test("new sessions exclude games that started before the session boundary", () => {
  const bounds = sessionBounds("current", 10000, 5000, "", "");
  assert.deepEqual(sessionMatches([{ started_at: 4000 }, { started_at: 5000 }, { started_at: 9000 }], bounds), [{ started_at: 5000 }, { started_at: 9000 }]);
});
test("diagnostics distinguish required Fullscreen setup from optional desktop components", () => {
  const output = { mode: "fullscreen", enabled: true, output: "game_bar", game_bar_installed: true, widget_installed: true, pinned: false, click_through: false };
  let rows = diagnosticChecks("CONNECTED", output, null, null);
  assert.equal(rows.find(r => r.id === "pin").level, "warn");
  assert.equal(rows.find(r => r.id === "clicks").level, "warn");
  assert.equal(rows.find(r => r.id === "timer").level, "info");
  rows = diagnosticChecks("CONNECTED", { ...output, mode: "borderless" }, null, null);
  assert.equal(rows.find(r => r.id === "components").level, "info");
  assert(!rows.some(r => r.id === "pin"));
});
test("failed capture or unavailable probes are never reported as healthy", () => {
  const rows = diagnosticChecks("WAITING_FOR_GAME", null, { enabled: true, icon_enabled: true, icon_status: "capture failure", icon_similarity: 0, error: "capture" }, null);
  assert(!rows.some(r => r.level === "ok" && ["display", "mode", "detector"].includes(r.id)));
  assert.equal(rows.find(r => r.id === "detector").level, "warn");
});
test("all new English and Turkish messages have matching keys and intact encoding", () => {
  assert.deepEqual(Object.keys(featureMessages.en).sort(), Object.keys(featureMessages.tr).sort());
  for (const language of Object.values(featureMessages)) for (const text of Object.values(language)) {
    assert(text.trim()); assert(!/[\uFFFD]|â€™|Ã¼|Ä±/.test(text), text);
  }
});
