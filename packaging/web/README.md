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
