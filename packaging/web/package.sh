#!/usr/bin/env bash
set -euo pipefail
echo 'error: standalone editor ZIP disabled: easyPDF web requires the authenticated web-portal host. Build the Docker stack with compose.yaml instead.' >&2
exit 2
