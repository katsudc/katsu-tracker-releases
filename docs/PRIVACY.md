# Privacy & network details

## Game session

The tracker connects to the existing Riot Client session and uses it for game state, available player identities, ranks, loadouts and personal match history. Riot sign-in remains in Riot Client. No external player-statistics account or API key is required.

## Local data

Preferences, content artwork and personal match-history snapshots are cached locally. Cached information helps the app remain useful when a refresh fails. Public preview screenshots in this repository use sample identities and data.

## Network services

| Service | Purpose |
| --- | --- |
| Riot services through the existing session | Game state, available ranks/loadouts, personal history and existing in-client social features. |
| VALORANT content API and its media host | Agent, map, rank and other content artwork. |
| GitHub Releases | Update discovery, installer downloads and release documentation. |
| Tracker.gg | Community lineup pages loaded when you request them in the Lineups view. |
| Windows media/audio session | Local Spotify metadata, playback and app-specific volume controls. |

External lineup pages are governed by their provider. Their availability and page behavior are outside this project's control.

## Reports and screenshots

Review screenshots and logs before sharing them. Remove Riot passwords, session tokens, authentication headers, personal Riot IDs, private messages and signing keys. Use [private security reporting](../SECURITY.md) for sensitive vulnerabilities.

[Back to the project](../README.md)
