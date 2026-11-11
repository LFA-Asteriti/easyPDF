<?php
declare(strict_types=1);

require_once __DIR__ . '/../src/bootstrap.php';

if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}
if ($argc < 4 || !in_array($argv[3], ['super_admin', 'admin', 'user'], true)) {
    fwrite(STDERR, "Uso: php bin/create-user.php email nome super_admin|admin|user [slug-organizzazione]\n");
    exit(2);
}
$email = mb_strtolower(trim($argv[1]));
$name = trim($argv[2]);
if (!filter_var($email, FILTER_VALIDATE_EMAIL) || $name === '') {
    fwrite(STDERR, "Email o nome non validi.\n");
    exit(2);
}
fwrite(STDOUT, 'Password (minimo 8 caratteri): ');
$password = trim((string) fgets(STDIN));
if (strlen($password) < 8) {
    fwrite(STDERR, "Password troppo corta.\n");
    exit(2);
}
$organizationId = null;
$organizationRole = null;
if ($argv[3] === 'admin') {
    $slug = trim((string) ($argv[4] ?? ''));
    $lookup = db()->prepare('SELECT id FROM organizations WHERE slug = ? AND is_active = 1');
    $lookup->execute([$slug]);
    $organizationId = $lookup->fetchColumn();
    if ($organizationId === false) {
        fwrite(STDERR, "Per un Admin indica lo slug di un’organizzazione attiva.\n");
        exit(2);
    }
    $organizationId = (int) $organizationId;
    $organizationRole = 'admin';
}
$stmt = db()->prepare('INSERT INTO users (email, display_name, password_hash, role, organization_id, organization_role) VALUES (?, ?, ?, ?, ?, ?)');
$stmt->execute([$email, mb_substr($name, 0, 120), password_hash($password, PASSWORD_DEFAULT), $argv[3], $organizationId, $organizationRole]);
fwrite(STDOUT, "Utente creato.\n");
