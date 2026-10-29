# Companion Suite · .50 milestone

This is an extension of the existing Tauri/React desktop application. It does not replace the tracker, updater, instalock, Spotify integration or Spike detector. The conceptual .47–.50 roadmap was completed together. Release `1.9.6-personal.51` includes the suite and a fix for normal startup after the recovery choice.

## Navigation and recovery

- **Ctrl+K**, while the app has focus, opens searchable commands. Use arrows, Enter and Escape. The header button remains available if a custom overlay hotkey already uses Ctrl+K.
- The Windows tray opens the existing window, toggles streamer/Spike/instalock preferences, displays the observed game state and exits. Options exposes minimize, close and startup behavior. Close-to-tray and start-minimized default off for existing users.
- An unclean prior exit offers normal or Safe Mode startup. Safe Mode pauses Spotify, Discord activity, Game Bar, optional overlays and effects for that run. It keeps core navigation and saved settings. Restart normally to restore integrations.
- Tools → Diagnostics previews an allowlisted report before saving a ZIP. It contains system/display/integration states and safe state events, **not original log files**, accounts, names, file paths, credentials or user notes. It does not upload a report.

## Personal performance

Statistics has Overview, Personal coach, Agents, Maps, Rank Journey and Act recap tabs. Existing daily/custom sessions and match cards remain available.

The source remains the user's existing Riot session and up to 20 accessible Competitive matches. Calculations concern the user's own normalized statistics. No additional opponent-performance provider or AI service is used.

- K/D, per-match K/D/A, ACS, ADR, HS% and win rate require their actual underlying values. Missing values remain unavailable.
- Opening kills/deaths require complete player event sets for every supplied round and an unambiguous first opposing-player elimination. Missing rounds, kill arrays, timestamps or tied earliest events do not produce an estimate. Clutches and attack/defense splits are not inferred.
- Coach comparisons use at least 10 matches for recent-five ACS comparisons and five matches per highlighted agent/map group. Results describe samples, not predictions.
- Historical match tiers and exact-match observed MMR updates drive Rank Journey. The current rank is never assigned retrospectively to old matches. RR changes are retained only when received for that exact match; old history can therefore have gaps.
- Act recap means **available history**, not a complete Act archive. Known season IDs can filter the sample. PNG creation reuses the match-card export helpers and is deferred while a match is active. Cards contain no account identity.
- Optional session goals cover matches, RR, duration and consecutive losses. Reminders require an explicit session start, default off, and wait until the lobby if reached during play. RR totals require actual RR changes for all session matches.

## Reference tools

Tools groups saved lineups, crosshairs, sensitivity profiles, notes, diagnostics and privacy. Large tools load when opened.

- Existing lineups retain URLs, timestamps, notes, favorites and filters. Context following is optional and applies only when map, own agent and side are all observed. It filters references; it does not play abilities or move the mouse.
- Version 1 `katsu-lineup-pack` JSON includes a name, creation timestamp and validated records. Import previews counts and supports Skip, Replace and Keep both. HTTPS hosts are allowlisted, arbitrary content is not executed, and the library is capped at 200 entries. There is no pack server.
- Crosshair profiles offer local CRUD, search, favorites and copying. Format checks are basic; users paste codes into VALORANT themselves.
- Sensitivity profiles calculate `DPI × sensitivity`, store favorites and up to 30 recent changes. They do not alter game settings.
- Match notes belong to an observed account and match. Agent/map reference notes, lineups, crosshairs and sensitivity profiles are shared across local accounts.
- Optional F8 moment marking uses the existing shortcut plugin. It stores a timestamp, own account/match ID and optional note, **not media**. Elapsed time is explicitly time since the tracker observed the match. No round is guessed. Disabled marking has no registered shortcut.

## Storage and privacy

No new server, telemetry or database was added. Versioned preferences and user-generated data use the existing WebView storage; native cache and recovery files use the existing application data directory. Existing .46 keys and schemas are retained, with additive optional fields and safe defaults.

Local accounts follow the connected Riot session; this is not a login manager. Personal native history was already separated by account. Optional account-specific instalock choices are applied only in the lobby, after identity is known. The account registry is bounded to 10 records.

Notes and markers are capped at 500 each, crosshairs/sensitivity at 100 each, goals at 10 accounts and lineups at 200. Undecodable tool storage preserves an original `.recovery` copy before fallback; if the copy cannot be made, overwriting the original is blocked.

Privacy controls require confirmation for scoped clearing. The broad reset describes the managed categories it resets. Riot authentication, VALORANT files, saved game presets, installed Game Bar components, certificates and signing keys are preserved. Cache reset is blocked while playing or refreshing.

Riot requests, public game artwork, GitHub updates, requested translations, user-opened guides and optional Discord activity can use the network. Spotify uses local Windows media controls; Game Bar uses the existing local bridge. Notes, markers and profiles are not uploaded.

## Regression boundary and stabilization

**Spike Timer visual detection logic was not modified.** The detector, capture/ROI/threshold/timing modules, Game Bar bridge/widget, media backend and updater backend are protected by baseline SHA-256 comparison. Normal startup retains their initialization order. Safe Mode gates optional startup outside those modules.

New analytics run on bounded history updates or when their page opens. Charts are SVG, no chart dependency was added. Diagnostics and recap images are created only on request. There are no new gameplay polling loops. Animations respect reduced motion; Safe Mode disables optional effects.

After this milestone, prioritize fixes, compatibility, startup/memory performance and user feedback. Do not extend the feature scope during stabilization.
