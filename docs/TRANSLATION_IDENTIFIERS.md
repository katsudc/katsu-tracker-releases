# Translation client identifiers

The chat translation adapter in `src-tauri/src/translate.rs` contains two Google Translate client constants inherited from the original application. They are not private credentials for the katsu project or a user's Google Cloud account.

Their provenance was checked against Google's public distribution on 28 October 2026:

- `KEY_HTML` matches the identifier in the JavaScript loaded by Google's [Translate Element widget](https://translate.google.com/translate_a/element.js?cb=googleTranslateElementInit).
- `KEY_PA` matches the identifier in Google's official [Google Translate Chrome extension](https://chromewebstore.google.com/detail/google-translate/aapbdbdomjkkjkaonfhkkikfgjllcleb), extension ID `aapbdbdomjkkjkaonfhkkikfgjllcleb`. The public package distributed by Google's extension-update service contains the same constant in its compiled client scripts.

No requests were made to authenticated APIs with these values to test account access, billing or permissions. The comparison was between already-public client code and the project's constants.

GitHub can identify these strings as Google API keys from their format. A secret-scanning alert with validity **Unknown** does not establish that a private project credential was exposed. The known public-client origin of these two constants does not make other Google API keys safe to publish. Personal Google Cloud/Gemini keys, service-account credentials, Riot session tokens and private update-signing material must remain outside the repository.

The adapter uses unofficial translation endpoints and includes keyless fallback attempts; availability is controlled by Google. Future changes to translation authentication should keep private credentials out of distributed client binaries and source code.
