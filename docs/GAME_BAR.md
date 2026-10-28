# Fullscreen Spike timer and Game Bar

## Choose the display mode

- **Windowed Fullscreen / Windowed:** the existing desktop countdown follows the game viewport and monitor DPI. Customize it in **Spike Timer → Windowed Fullscreen → Edit overlay**.
- **Fullscreen:** the optional pinned Game Bar widget displays the same countdown. Its font size and background opacity are saved independently. Press **Win + G** and drag **katsu’s Tracker** to position it.

The existing visual detector, HUD capture area, thresholds, polling, fuse duration and round-end reset are shared and unchanged. Audio calibration is not required. Colors reflect estimated time left for a full defuse, half defuse or neither; actual defuse progress is not detected.

## First setup

Open **Spike Timer** and approve Fullscreen component setup if prompted. Nothing is downloaded or installed before approval; **Not now** dismisses the prompt. The widget and public publisher certificate are bundled. A missing Xbox Game Bar is obtained through Microsoft Store/WinGet after approval, with a displayed error and Store fallback if Windows policy or installation availability prevents it.

The current direct-distribution widget uses a self-signed publisher certificate. Windows may request administrator approval to trust that public certificate in **Local Computer → Trusted People**, affecting this computer’s accounts. The helper validates the package signer and the certificate pinned into the desktop build before registering trust. Only certificate registration is elevated; package registration stays in the original user’s account. UAC cancellation stops setup. No certificate is added to Trusted Root, no private signing key is distributed, and the tracker does not need to stay elevated.

Enable Xbox Game Bar in Windows if Win + G does not open. Pin **katsu’s Tracker** once. While Game Bar is open, the widget shows a 45.0 positioning preview. The application reports the SDK’s actual click-through value; turn it on in Game Bar and close the Game Bar menu to pass clicks through the active countdown. The app does not silently override that Windows preference.

## Small window, no invisible input area

The widget fits its window to the number rather than expanding to an entire display. It never recenters a window that the user has dragged. When no countdown is active and the Game Bar menu is closed, its actual window is minimized, not merely made transparent. The desktop restores it on the next plant. A stale-data watchdog also minimizes it if the tracker stops.

The bridge reads Game Bar.Visible and the SDK foreground state to restore the positioning preview in Win + G. It uses supported Game Bar restore/minimize URIs. It does not change the game’s window styles, fullscreen optimization settings, background recording, Developer Mode or game memory, and does not inject game input.

## Data and configuration

The widget reads a small presentation-only JSON file in its own package LocalState: a Windows performance-counter deadline, appearance and display state. It contains no Riot credentials, player identities or match history. Its heartbeat reports visibility, pinned/click-through status and rendering health. Stale data is hidden within three seconds.

The desktop reads the current account’s and shared WindowsClient GameUserSettings.ini files without modifying them. It uses FullscreenMode when present, otherwise LastConfirmedFullscreenMode from the same game-settings section; PreferredFullscreenMode never determines the applied mode. The windowed editor and widget styles use the existing saved preferences.

## Build and validation

See [Building from source](BUILDING.md). Build the widget with `scripts/build_gamebar.ps1 -SignWithLocalTestCertificate` before the desktop. This requires Visual Studio 2022 C++/CX tools and a Windows SDK. Microsoft Game Bar SDK 7.3.2607010 is fetched from NuGet with a pinned SHA-256. Its SDK license and the Microsoft sample attribution are preserved. Compiler output, packages and generated public certificates are ignored by Git; the non-exportable private key stays in the developer’s Windows certificate store.

Local validation covered the Spike regression suite, sidebar/settings UI and independent appearance, a real Fullscreen plant countdown, and native WinRT restore/minimize behavior. Live widget checks confirmed number-sized bounds (90×42 at font 31, 222×105 at font 84) and actual minimization after reset or stale data. Tests marked ignored need the relevant Windows/game/widget environment and run only when explicitly selected.

## Microsoft Store distribution

The widget currently ships in the desktop installer. A future Microsoft Store widget would use Store signing and would not require this public-certificate trust step. The package identity/product ID and distribution flow would need to be configured. Microsoft’s Game Bar documentation requires a Store policy exception for direct communication with an unpackaged desktop application or links to its external download; that must be resolved before a Store submission.

References: [Game Bar widget APIs](https://learn.microsoft.com/en-us/xbox/game-bar/api/xgb-widget), [widget control URIs](https://learn.microsoft.com/en-us/xbox/game-bar/api/xgb-widgetcontrol), [Game Bar visibility](https://learn.microsoft.com/en-us/uwp/api/windows.gaming.ui.gamebar.visible), [desktop communication](https://learn.microsoft.com/en-us/xbox/game-bar/guide/communicating-apps), [distribution](https://learn.microsoft.com/en-us/xbox/game-bar/guide/distribution), [certificate trust](https://learn.microsoft.com/en-us/windows/msix/msix-troubleshooting-guide).
