# Features

The [Companion Suite milestone](COMPANION_SUITE.md) adds command search, tray/recovery, personal performance views, local player tools and privacy controls. Version .51 also fixes the normal-start recovery transition.

## Live match overview

The match view presents both teams with names, agents and available ranks. Rank artwork is cached when available, with a visual fallback when it cannot load. Confirmed unranked accounts and unavailable ranks are labelled separately.

Live cards do not contain K/D, KDA, ACS, HS%, ADR, win rate or historical performance. Existing weapon-skin views remain accessible from player cards when the game supplies the loadout.

![Live match preview using sample data](assets/match-overview.png)

## Instalock

Open **Instalock** to choose a default agent or a map-specific preference. The delay control sits directly below the portraits. The existing portrait grid and character artwork help you see the current selection. Automatic locking has an on/off control and a configurable delay. Agent preferences persist between launches.

## Visual Spike timer

The dedicated **Spike Timer** page controls a compact countdown detected from the planted-Spike HUD icon. Windowed modes use the desktop overlay and its visual editor; Fullscreen uses an optional Game Bar widget. Change its size/background independently and position it by dragging it in Win + G. Component setup asks for consent before installation. The widget minimizes its actual window when idle and restores it on plant. Preferences save automatically, and the countdown resets at round end. There is no audio-calibration step.

Countdown colors reflect estimated time remaining for a full or half defuse, rather than detecting whether someone has started defusing.

## Spotify controls

The compact player expands on hover. It shows the current track, artist, artwork and available progress, with previous/play/next and a Spotify-only volume slider. It uses the local Windows Spotify media session and does not ask you to sign in inside the tracker.

![Agent selection and Spotify preview using sample data](assets/agent-and-music.png)

## Personal match history

Your own accessible Competitive history can be viewed as the last 5, 10 or 20 matches. Available metrics include K/D/A, ACS, HS% and damage per round, with agent/map breakdowns and lightweight performance trends. A fresh completed match can produce a post-match summary.

These are recent, personal figures, not lifetime statistics or a Tracker Score. Missing fields are left unavailable. The current Riot session supplies the data; cached information survives temporary refresh failures.

## Community lineups

The Lineups view uses agent, map and side filters to open Tracker.gg community guides. The external page loads after an explicit request. Leaving the view unloads its playback; filter choices persist. Content availability, ads and guide attribution are controlled by Tracker.gg.

## Themes, language and updates

Choose Mint, Blue, Violet, Rose or Amber. The interface supports English and Turkish, with preferences saved between launches. The startup update check is visible. Choose when to download and install a signed release; download progress appears in the app, installation runs silently and the tracker restarts automatically.

## Personal lineup library

The **Lineups → My library** tab keeps up to 200 video bookmarks locally, with an agent, map, side, notes, favorite status and a saved start time. Search titles/notes or filter by your current agent/map/side choices. Add, edit and remove entries without loading a third-party page. Video links open only after a click. YouTube links open at the saved time; other providers show the timestamp with the note.

## Daily sessions

The **Statistics** page includes Today, Yesterday, Last 7 days, This session and a custom date range. Session boundaries are stored separately for each account. Totals use the available Competitive history (up to 20 matches) and local calendar dates; missing dates are excluded. Starting a new session does not delete any history. No estimated RR changes are invented.

## Shareable match cards

Open a completed match and choose **Create match card** to preview a themed 1080 × 660 PNG containing your own result, agent, map and available metrics. Save it through the Windows file picker or copy the image to the clipboard. Names are optional and are disabled when streamer-name hiding is active. Creating the card does not upload it anywhere.

## Setup and tools

**Tools** contains a three-step first-run guide, settings transfer, streamer mode and live connection/overlay diagnostics. The setup guide can be opened again. Changing the setup instructions does not modify VALORANT's display mode. Selecting a monitor moves only the tracker window; the timer still follows the actual game window. Fullscreen component installation always requires the existing explicit approval.

Settings files contain an allowlisted, versioned set of preferences: language, theme, hotkey, window behavior, agent preferences, lock delay, Discord preference, timer toggle/appearance and streamer preferences. A preview is shown before import, invalid files are rejected, and imports are disabled during agent select or a live match. Failed native updates attempt to restore the previous values and report incomplete recovery honestly. Credentials, account sessions, lineup notes, match history, device coordinates and certificate trust are excluded. Game Bar position remains managed by Windows.

Streamer mode can mask roster/profile/scoreboard/player-detail names, hide chat, quiet toast notifications and hide the Spotify player. Discord activity is paused while the mode is active. These choices affect this tracker, not VALORANT or broadcasting software.

Diagnostics read the existing connection, game-display, HUD capture, timer, component, pin and click-through status. They run while the page is mounted and provide relevant next steps; they do not install software or change Windows settings.

[Setup guide](GETTING_STARTED.md) · [Troubleshooting](TROUBLESHOOTING.md) · [Back](../README.md)
