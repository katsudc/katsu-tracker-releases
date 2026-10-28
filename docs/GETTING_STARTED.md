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
| Spike timer | Enable **Spike timer** and use **Edit overlay** to position or restyle the countdown. |

Preferences are saved automatically.

## Spike timer

The countdown follows the game viewport and monitor, including mixed-DPI and non-native resolutions. **Borderless Windowed / Windowed Fullscreen** is the most reliable display mode; true exclusive fullscreen can hide external windows. The detector watches the planted-Spike HUD icon near the top of the game view; keep that HUD visible. The countdown appears on the game monitor and resets at round end.

Open **Options → Spike timer → Edit overlay** to drag the seconds to a new position. Use the corner handle or **Text size** slider to resize, and **Background opacity** for contrast. Arrow keys move the focused timer; Shift + arrow moves farther. Changes save automatically. **Reset position & style** restores the default.

The countdown is a timing estimate. Its colors indicate whether the remaining time allows a full defuse, only a half defuse, or neither; it does not observe actual defuse progress. Detection is visual and does not require audio calibration.

## Spotify

Open the Windows Spotify desktop app and start a track. Hover over the mini-player to expand the artwork, progress, playback controls and volume slider. The volume control targets Spotify's audio session.

## Lineups

Open **Lineups**, select an agent, map and side, then press **Show lineups**. Guides load from Tracker.gg. Expanded view and **Open in browser** are available when you want more room or the embedded view cannot load.

## Updating

The tracker visibly checks for updates at startup. When one is available, close VALORANT and choose **Update now**. Follow the real download progress, signature verification, silent installation and automatic restart in the app. No setup wizard is opened. Existing preferences are retained. Older clients use their previous update flow once to install 1.9.6-personal.36.

[Need help?](TROUBLESHOOTING.md) · [Back to the project](../README.md)
