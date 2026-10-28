# Security

## Report privately

Use this repository's **Security → Report a vulnerability** flow for sensitive security issues:

[Create a private vulnerability report](https://github.com/katsudc/katsu-tracker-releases/security/advisories/new)

Describe the affected version, reproduction steps and impact. Include only the minimum information needed to investigate. Remove real credentials, session tokens, personal player data and private keys.

Do not publish a working credential, token or a sensitive vulnerability in a public issue. General application bugs can use the [bug-report template](https://github.com/katsudc/katsu-tracker-releases/issues/new?template=bug_report.yml).

## Update integrity

Application updates use updater-signature verification through the project's release channel. Download installers from this repository's [Releases](https://github.com/katsudc/katsu-tracker-releases/releases) page.

## Public client identifiers and private credentials

The two inherited Google Translate client constants have verified public Google-client provenance, documented in [Translation client identifiers](docs/TRANSLATION_IDENTIFIERS.md). Review alerts individually; this exception does not apply to personal Google Cloud/Gemini keys, service accounts, account/session tokens, or updater private signing keys. Never publish those credentials, local credential files or environment files.
