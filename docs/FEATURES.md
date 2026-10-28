# Features

## Live match overview

The match view presents both teams with names, agents and available ranks. Rank artwork is cached when available, with a visual fallback when it cannot load. Confirmed unranked accounts and unavailable ranks are labelled separately.

Live cards do not contain K/D, KDA, ACS, HS%, ADR, win rate or historical performance. Existing weapon-skin views remain accessible from player cards when the game supplies the loadout.

![Live match preview using sample data](assets/match-overview.png)

## Agent preferences

Choose a default agent or a map-specific preference. The existing portrait grid and character artwork help you see the current selection. Automatic locking has an on/off control and a configurable delay. Agent preferences persist between launches.

## Visual Spike timer

The timer follows the planted-Spike icon in the game HUD and displays a small, centered countdown on the game monitor. It resets at round end. This system uses visual detection; there is no sound-recording setup.

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

Choose Mint, Blue, Violet, Rose or Amber. The interface supports English and Turkish, with preferences saved between launches. Updates are downloaded through the existing GitHub release channel and verified with the updater signature.

[Setup guide](GETTING_STARTED.md) · [Troubleshooting](TROUBLESHOOTING.md) · [Back](../README.md)
