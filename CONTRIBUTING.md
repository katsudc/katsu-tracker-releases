# Contributing

Thanks for helping improve katsu's Tracker.

## Useful contributions

- Clear bug reports with reproducible steps.
- Focused feature requests that explain the problem being solved.
- Documentation corrections, accessibility feedback and translation feedback.
- Focused source-code, testing and documentation improvements through a pull request.

Search existing issues before opening a new one. Keep unrelated ideas in separate issues and describe the expected behavior.

## Current repository scope

This repository contains the Windows desktop application source, releases, documentation and issue tracking. See [Building from source](docs/BUILDING.md) for setup and validation commands. Keep changes focused, explain observable behavior and include the checks relevant to your change.

Documentation is written in English for the public project. The application itself continues to support English and Turkish.

## Keep reports clean

Use sample data or redact player identities in screenshots. Never include Riot credentials, tokens, authentication headers, signing keys or private chat messages. Security reports belong in the private process described in [SECURITY.md](SECURITY.md).

Please follow the [community guidelines](CODE_OF_CONDUCT.md).

Before submitting or publishing source, run `powershell -NoProfile -File scripts/check_source_secrets.ps1`. It reports file locations without printing credential values; only the two verified public Translate client constants are allowlisted by exact fingerprint.
