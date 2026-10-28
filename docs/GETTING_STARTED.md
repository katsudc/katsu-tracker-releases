# Getting started

## Requirements

- A Windows x64 PC with a working VALORANT installation.
- An existing Riot Client session for match and account information.
- Internet access for live game data, content assets and update downloads.
- The Windows Spotify desktop app, if you want music controls.

## Install

1. Download `Katsu-Tracker-setup.exe` from the [latest release](https://github.com/katsudc/katsu-tracker-releases/releases/latest).
2. Run the installer and launch **katsu's Tracker**.
3. Open Riot Client and VALORANT normally. Sign in through Riot Client, not through the tracker.
4. Wait for the connection indicator. Your profile is identified when the game session becomes available.

No external statistics API key or separate tracker account is required.

## Your first setup

| Setting | Where to start |
| --- | --- |
| Overlay shortcut | **F2** shows or hides the main window. |
| Default agent | Open **Options → Agent** and choose a portrait. |
| Per-map agent | Choose the preferred agent for each map in the same panel. |
| Lock delay | Set the delay in the agent settings; keep the auto-lock switch off when you do not want it to run. |
| Theme & language | Open the settings tab to choose an accent and English or Turkish. |
| Window position | Use the movable window mode and place the main window where it suits your setup. |
| Spike timer | Enable the single **Spike timer** switch in settings. |

Preferences are saved automatically.

## Spike timer

Choose **Borderless Windowed / Windowed Fullscreen** in VALORANT. The detector watches the planted-Spike HUD icon near the top of the game view; keep that HUD visible. The countdown appears on the game monitor and resets at round end.

The countdown is a timing estimate. Its colors indicate whether the remaining time allows a full defuse, only a half defuse, or neither; it does not observe actual defuse progress. Detection is visual and does not require audio calibration.

## Spotify

Open the Windows Spotify desktop app and start a track. Hover over the mini-player to expand the artwork, progress, playback controls and volume slider. The volume control targets Spotify's audio session.

## Lineups

Open **Lineups**, select an agent, map and side, then press **Show lineups**. Guides load from Tracker.gg. Expanded view and **Open in browser** are available when you want more room or the embedded view cannot load.

## Updating

Updates download in the background and are checked using the updater signature. When the update is ready, close VALORANT and use the update notification, or let the tracker install it the next time it opens. Keep the existing installation to retain your preferences.

[Need help?](TROUBLESHOOTING.md) · [Back to the project](../README.md)
