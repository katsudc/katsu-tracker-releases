# Local build fix

Source: tauri-winres 0.3.5 from the locked Cargo registry package (MIT).
The only implementation change removes backslash escaping for apostrophes
in helpers::escape_string. Microsoft RC preserves that backslash literally
in ProductName/FileDescription; the intended name is katsu's Tracker.
Other escaping and resource generation are unchanged. Included as a local
Cargo patch so source archives reproduce the fix without registry edits.
