# Troubleshooting

## The tracker is waiting for VALORANT

Open Riot Client, sign in normally and start VALORANT. The tracker needs an existing session to identify your account. If the session changed, use the reconnect control or restart the tracker.

Do not share Riot passwords, session tokens or authentication headers in an issue report.

## A player's rank is unavailable

Ranks are displayed only when the session supplies usable match or MMR data. **Rank unavailable** is different from a confirmed **Unranked** account. The app does not estimate ranks from match performance or an external stats service.

## Personal history is incomplete or old

The history covers up to 20 accessible Competitive matches. Initial loading and delayed post-match availability depend on the game data source. Use **Refresh** when appropriate. If a request fails or reaches a rate limit, existing cached information remains visible.

## The Spike countdown does not appear

1. Open **Spike Timer** and enable the timer.
2. Keep the HUD visible and the game in the foreground. Test a new plant after starting the application.
3. In windowed modes, check the game monitor and use **Edit overlay → Reset position & style** if needed.
4. In Fullscreen, approve component setup and pin **katsu’s Tracker** in **Win + G**. Check that Xbox Game Bar is enabled in Windows if Win + G does not open.
5. To reposition the Fullscreen widget, drag it while Win + G is open. Check the real click-through status in **Spike Timer → Game Bar** and close the Game Bar menu after editing.

The idle widget is minimized, not merely transparent. If a widget seems stuck over another app, check that the Game Bar menu is closed and click-through is enabled. Windows policy or Store/WinGet restrictions can prevent component setup; use the displayed error rather than repeatedly adding certificates.

The detector uses the planted-Spike HUD icon. Changing microphone or recording settings will not help this visual detector. Defuse progress is not measured.

## Spotify is empty or controls are unavailable

Use the Windows Spotify desktop app and start a track. Availability of track metadata, progress and transport controls depends on the active Windows media session. Spotify volume control also requires an accessible Spotify audio session.

## Lineup videos do not load

Press **Show lineups** after choosing the filters. If the embedded page is unavailable, try **Open in browser**. The guide catalog and playback are provided by Tracker.gg and may require its own page interactions.

## An update does not appear

Restart the tracker to trigger the visible startup check. Choose **Update now** to download and install, and close VALORANT first. A failed download or installation offers a retry. If automatic restart cannot be confirmed, use **Open tracker**. You can also download the installer from the [latest release](https://github.com/katsudc/katsu-tracker-releases/releases/latest).

## Still stuck?

[Open a bug report](https://github.com/katsudc/katsu-tracker-releases/issues/new?template=bug_report.yml) with the tracker version, Windows version, steps to reproduce and a redacted screenshot if useful. Avoid uploading unreviewed logs, personal Riot IDs, tokens or private keys.

[Support](../SUPPORT.md) · [Back](../README.md)
