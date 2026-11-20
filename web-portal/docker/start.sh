#!/bin/sh
set -eu

php /var/www/html/bin/migrate-connected-app.php
exec apache2-foreground
