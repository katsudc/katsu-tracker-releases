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
| Default agent | Open **Instalock** and choose a portrait. |
| Per-map agent | Choose the preferred agent for each map in the same panel. |
| Lock delay | Set the delay in the agent settings; keep the auto-lock switch off when you do not want it to run. |
| Theme & language | Open **Options** to choose an accent and English or Turkish. |
| Window position | Use the movable window mode and place the main window where it suits your setup. |
| Spike timer | Open **Spike Timer**. Use **Edit overlay** for windowed modes, or the Game Bar section for Fullscreen. |

Preferences are saved automatically.

## Spike timer

In windowed modes, the countdown follows the game viewport and monitor, including mixed-DPI and non-native resolutions. In Fullscreen, it uses the optional Game Bar widget. Detection watches the planted-Spike HUD icon; keep the HUD visible. The countdown resets at round end.

Open **Spike Timer → Windowed Fullscreen → Edit overlay** to drag the seconds, resize them or change background opacity. Arrow keys move the focused timer; Shift + arrow moves farther. Changes save automatically.

For Fullscreen, approve component setup if offered. Nothing is downloaded or installed before approval. The widget and public signing certificate are bundled; Windows may request administrator approval to add that certificate to the computer’s Trusted People store. If Xbox Game Bar is missing, setup obtains it through Microsoft Store/WinGet. The private signing key is never distributed.

Press **Win + G** and pin **katsu’s Tracker** once. Drag it while Game Bar is open to set its position; the menu shows a 45.0 preview. Enable Game Bar’s mouse click-through and close the menu to pass clicks through the active countdown. The application shows the actual Windows-reported setting. When no countdown is running, the widget’s real window is minimized so no invisible input area remains. Font size and background are saved independently from the windowed editor.

See [Game Bar setup and architecture](GAME_BAR.md) for details.

The countdown is a timing estimate. Its colors indicate whether the remaining time allows a full defuse, only a half defuse, or neither; it does not observe actual defuse progress. Detection is visual and does not require audio calibration.

## Spotify

Open the Windows Spotify desktop app and start a track. Hover over the mini-player to expand the artwork, progress, playback controls and volume slider. The volume control targets Spotify's audio session.

## Lineups

Open **Lineups**, select an agent, map and side, then press **Show lineups**. Guides load from Tracker.gg. Expanded view and **Open in browser** are available when you want more room or the embedded view cannot load.

## Updating

The tracker visibly checks for updates at startup. When one is available, close VALORANT and choose **Update now**. Follow the real download progress, signature verification, silent installation and automatic restart in the app. No setup wizard is opened. Existing preferences are retained. Older clients use their previous update flow once to install 1.9.6-personal.36.

[Need help?](TROUBLESHOOTING.md) · [Back to the project](../README.md)
