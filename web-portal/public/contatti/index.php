<?php
declare(strict_types=1);

require dirname(__DIR__, 2) . '/src/bootstrap.php';
require_user();
header('Location: /supporto/', true, 302);
exit;
