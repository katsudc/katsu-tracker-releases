# Release notes

## 1.9.6-personal.51

- Publish the Companion Suite expansion: app-focused command palette, system tray, temporary Safe Mode and sanitized diagnostic ZIP export.
- Add own-performance coach, map/agent drilldowns, Rank Journey, recap images and optional session goals.
- Add portable lineup packs, local notes/moment timestamps, crosshair and sensitivity profiles, account preferences and privacy controls.
- Fix a Windows deadlock when selecting Start normally after recovery by creating the existing overlay outside the synchronous IPC handler.
- Preserve existing preferences, the visual Spike detector, Game Bar, Spotify and signed updater behavior.


## 1.9.6-personal.50 — local testing, unpublished

- Complete the .47–.50 Companion Suite expansion without changing the visual Spike detector.
- Add app-focused command search, a Windows tray menu, temporary Safe Mode and sanitized diagnostic ZIP export.
- Add personal coach, agent/map drilldowns, rank journey, available-history recap images and opt-in session goals.
- Extend saved lineups with portable validated packs and optional observed-game filters.
- Add local notes/moment timestamps, crosshair and sensitivity profiles, optional account instalock preferences and confirmed privacy controls.
- Preserve .46 storage formats, updater, Spotify and existing desktop/Game Bar timer implementations.
- Enter feature freeze: subsequent work prioritizes reliability, performance and user feedback.


## 1.9.6-personal.45

- Add the optional Game Bar Spike timer for Fullscreen; windowed modes keep the desktop overlay.
- Ask before downloading or installing Fullscreen support. Bundle the widget and public publisher certificate; Windows administrator approval can still be required.
- Restore the widget when a plant is detected and minimize its actual window when idle, preventing an invisible rectangle from blocking clicks.
- Add dedicated Instalock and Spike Timer pages below Statistics.
- Keep the windowed visual editor and add independent Game Bar text-size/background settings, with Win + G drag instructions.
- Place Lock Delay immediately below the agent portraits.
- Preserve visual icon detection, round-end reset, existing preferences and the other tracker features.

## 1.9.6-personal.38

- Remove the legacy bottom-right update button and its unused translations.
- Keep the new update notification as the single update entry point, with the same progress, silent installation and automatic restart behavior.


## 1.9.6-personal.37

- Test release for the signed updater introduced in 1.9.6-personal.36.
- Verify startup discovery, download progress, silent installation, automatic restart and saved preferences with a real version upgrade.
- No application-feature, gameplay, visual detector or overlay behavior changes.


## 1.9.6-personal.36

- Show startup update discovery and let the user choose when to download and install.
- Display actual download bytes/percent, verify the signed package, install silently, and reopen the tracker automatically.
- Keep an independent progress window visible while the installed app is replaced; provide retry/reopen controls on failure.
- Add a visual Spike overlay editor with draggable position, keyboard placement, text size, background opacity, reset and automatic saving.
- Position the countdown relative to the physical game viewport across resolutions and mixed-DPI monitors, and refresh its native topmost order without activating it.
- Preserve the working icon detector, round reset, defuse color thresholds and existing preferences. True exclusive-fullscreen composition remains controlled by Windows.
- Publish the application source and Windows build instructions. GitHub generates the language breakdown from the repository.


Installers and complete version history are available in [GitHub Releases](https://github.com/katsudc/katsu-tracker-releases/releases).

## 1.9.6-personal.35

- Reduced redraw work in the agent settings and isolated portrait grids from hover updates.
- Simplified agent artwork transitions and removed multi-layer glitch effects.
- Added bounded decoded-preview caching, offscreen map-image deferral and smaller map thumbnails.
- Kept default/per-map selection immediate and preserved visual Spike detection.

## 1.9.6-personal.34

- Simplified live player cards to names, agents and ranks with artwork.
- Separated confirmed unranked accounts from unavailable ranks.
- Removed opponent performance scouting, roster historical-stat lookups, the external statistics provider and API-key settings.
- Retained personal history from the existing Riot session.
- Preserved visual Spike detection, saved preferences, agent/map selection and Spotify controls.

## 1.9.6-personal.33

- Added a cached personal dashboard, 5/10/20-match history and detailed personal statistics.
- Added agent/map breakdowns, lightweight charts and post-match summaries.
- Added provider and experimental-scoping work that was subsequently removed in personal.34.

## 1.9.6-personal.32

- Added the Lineups view with agent, map and attack/defense filters for Tracker.gg guides.
- Added expanded viewing and a browser fallback.

## 1.9.6-personal.31

- Refined the desktop layout and navigation.
- Improved the Spotify mini-player and progressive data loading.

## 1.9.6-personal.30

- Introduced the larger desktop interface, accent themes and Spotify integration.

## 1.9.6-personal.29

- Established the signed automatic-update distribution channel.
