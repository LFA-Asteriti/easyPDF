# easyPDF fonts

This directory contains the multilingual fonts embedded by easyPDF. Builds read
`fonts/manifest.txt` directly; no font repository, download or environment
variable is required.

Every manifest row records the original upstream source, OFL-1.1 licence,
licence file and SHA-256 of the unmodified font binary. The same files are
listed individually in the project root `ATTRIBUTION.toml`.

The directory contains all 22 approved font binaries formerly supplied by the
external font input. The manifest activates the 21 faces currently used by
easyPDF; LXGW WenKai Lite is retained as an audited OFL reserve for future use
without adding it to application binaries. The native application embeds all approved manifest rows. The web
build embeds only the smaller faces required by its supported interface scripts
so the WASM remains within common hosting limits. `Noto Sans CJK SC` is active
for the native Simplified Chinese interface; `LXGW WenKai Lite` remains stored
but inactive. Both were explicitly approved by the project owner on 2026-11-20.

When changing this bundle:

1. obtain the font from the original source recorded in the manifest;
2. keep the upstream OFL text in the same family directory;
3. update the manifest SHA-256 and the matching `ATTRIBUTION.toml` entry;
4. run `cargo xtask assets --write`, `cargo xtask assets`, the font tests,
   native checks and the WASM check.
