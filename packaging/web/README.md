# Hosting easyPDF web

The editor is part of the authenticated easyPDF portal. Do not publish the
editor WASM and JavaScript as a standalone static site: that would bypass
account, organisation, cover-kit and document-sharing access controls.

For a local integration test, from the repository root run
`docker compose up --build -d`, then open `http://localhost:8385/` and sign in.
The editor is served at `/editor/` by the same-origin portal. The checked-in
`compose.yaml` contains development-only credentials and must not be deployed
unchanged to a public server. The standalone ZIP command deliberately fails
until a secure distributable for a configured portal is reviewed.

## Hosting inside an authenticated page

A trusted page on the same easyPDF installation can host the editor in its own
document. Its canvas must have `id="easypdf" data-host="page"`; the page loads
the editor's generated JavaScript and WASM after registering event listeners.
This does not make the editor public: the page and its assets remain behind the
portal's normal session checks.

The canvas dispatches `easypdf:ready`, `easypdf:save`, `easypdf:dirty` and
`easypdf:failed` as DOM CustomEvents. Their `detail` contains the message
fields without `type`. To open a PDF, dispatch `easypdf:open` on the canvas
with `detail: { name, bytes }`, where `bytes` is an ArrayBuffer or Uint8Array.
Register the ready listener before loading the editor. The save event carries
`{ name, saveAs, bytes }`; the host page is responsible for storing those bytes.
The ready detail includes `version: "1"`, `appVersion` and `file: false`.

`?host=parent` takes precedence over `data-host="page"` and exchanges the
same named messages with a same-origin framing window using postMessage. An
unframed or cross-origin parent receives no save bridge, so saves download
normally. Files received from either host transport are limited to 512 MiB
before copying, and file names are cleaned and limited to 255 characters.
