<?php
declare(strict_types=1);

require_once __DIR__ . '/../src/bootstrap.php';

if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}
if (env_value('APP_ENV', 'production') === 'production') {
    fwrite(STDERR, "I dati di test non possono essere creati in produzione.\n");
    exit(2);
}
$password = (string) env_value('TEST_PASSWORD', 'EasyPDF-Test-2026!');
if (strlen($password) < 8) {
    fwrite(STDERR, "TEST_PASSWORD deve contenere almeno 8 caratteri.\n");
    exit(2);
}

$pdo = db();
$pdo->beginTransaction();
try {
    $pdo->prepare(
        "INSERT INTO organizations (name, slug, is_active) VALUES ('Organizzazione Demo', 'organizzazione-demo', 1) "
        . "ON DUPLICATE KEY UPDATE name = VALUES(name), is_active = 1"
    )->execute();
    $organization = $pdo->query("SELECT id FROM organizations WHERE slug = 'organizzazione-demo'")->fetchColumn();
    if ($organization === false) {
        throw new RuntimeException('Impossibile creare l’organizzazione di test.');
    }
    $upsert = $pdo->prepare(
        'INSERT INTO users (email, display_name, password_hash, role, organization_id, organization_role, is_active) '
        . 'VALUES (?, ?, ?, ?, ?, ?, 1) ON DUPLICATE KEY UPDATE display_name = VALUES(display_name), '
        . 'password_hash = VALUES(password_hash), role = VALUES(role), organization_id = VALUES(organization_id), '
        . 'organization_role = VALUES(organization_role), is_active = 1'
    );
    $hash = password_hash($password, PASSWORD_DEFAULT);
    $accounts = [
        ['admin@example.test', 'Super Admin', 'super_admin', null, null],
        ['user@example.test', 'Utente personale', 'user', null, null],
        ['org.admin@example.test', 'Amministratore organizzazione', 'admin', (int) $organization, 'admin'],
        ['org.user@example.test', 'Membro organizzazione', 'user', (int) $organization, 'member'],
    ];
    foreach ($accounts as [$email, $name, $role, $organizationId, $organizationRole]) {
        $upsert->execute([$email, $name, $hash, $role, $organizationId, $organizationRole]);
    }
    $pdo->commit();
} catch (Throwable $error) {
    if ($pdo->inTransaction()) {
        $pdo->rollBack();
    }
    fwrite(STDERR, $error->getMessage() . "\n");
    exit(1);
}

fwrite(STDOUT, "Account di test creati. Password comune: {$password}\n");
