<?php
declare(strict_types=1);

const EASYPDF_ROOT = __DIR__ . '/..';

function load_environment_file(): void
{
    $path = EASYPDF_ROOT . '/.env';
    if (!is_file($path) || !is_readable($path)) {
        return;
    }
    foreach (file($path, FILE_IGNORE_NEW_LINES | FILE_SKIP_EMPTY_LINES) ?: [] as $line) {
        $line = trim($line);
        if ($line === '' || str_starts_with($line, '#') || !str_contains($line, '=')) {
            continue;
        }
        [$key, $value] = array_map('trim', explode('=', $line, 2));
        if (!preg_match('/^[A-Z][A-Z0-9_]*$/', $key) || getenv($key) !== false) {
            continue;
        }
        if (strlen($value) >= 2 && (($value[0] === '"' && str_ends_with($value, '"')) || ($value[0] === "'" && str_ends_with($value, "'")))) {
            $value = substr($value, 1, -1);
        }
        putenv("{$key}={$value}");
        $_ENV[$key] = $value;
    }
}

function env_value(string $key, ?string $default = null): ?string
{
    $value = getenv($key);
    return $value === false || $value === '' ? $default : $value;
}

function db(): PDO
{
    static $pdo = null;
    if ($pdo instanceof PDO) {
        return $pdo;
    }
    $host = env_value('DB_HOST', '127.0.0.1');
    $port = env_value('DB_PORT', '3306');
    $name = env_value('DB_NAME', 'easypdf');
    $user = env_value('DB_USER', 'easypdf');
    $password = env_value('DB_PASSWORD', '');
    $dsn = "mysql:host={$host};port={$port};dbname={$name};charset=utf8mb4";
    $pdo = new PDO($dsn, $user, $password, [
        PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION,
        PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
        PDO::ATTR_EMULATE_PREPARES => false,
    ]);
    return $pdo;
}

function storage_path(string $suffix = ''): string
{
    $base = rtrim((string) env_value('EASYPDF_STORAGE', EASYPDF_ROOT . '/storage'), '/\\');
    return $suffix === '' ? $base : $base . DIRECTORY_SEPARATOR . ltrim($suffix, '/\\');
}

function ensure_directory(string $path): void
{
    if (!is_dir($path) && !mkdir($path, 0750, true) && !is_dir($path)) {
        throw new RuntimeException('Impossibile creare la cartella di archiviazione.');
    }
}

function start_secure_session(): void
{
    if (PHP_SAPI === 'cli' || session_status() === PHP_SESSION_ACTIVE) {
        return;
    }
    session_name('easypdf_session');
    session_set_cookie_params([
        'lifetime' => 0,
        'path' => '/',
        'secure' => env_value('APP_ENV', 'production') === 'production',
        'httponly' => true,
        'samesite' => 'Lax',
    ]);
    session_start();
}

function security_headers(bool $allowSameOriginScripts = false): void
{
    if (PHP_SAPI === 'cli' || headers_sent()) {
        return;
    }
    header('X-Content-Type-Options: nosniff');
    header('X-Frame-Options: DENY');
    header('Referrer-Policy: same-origin');
    header('Permissions-Policy: camera=(), microphone=(), geolocation=()');
    $scriptSource = $allowSameOriginScripts ? "'self'" : "'none'";
    header("Content-Security-Policy: default-src 'self'; img-src 'self' data:; style-src 'self'; script-src {$scriptSource}; form-action 'self'; frame-ancestors 'none'; base-uri 'self'");
}

function validate_configuration(): void
{
    $secret = (string) env_value('APP_SECRET', '');
    if (env_value('APP_ENV', 'production') === 'production' && (strlen($secret) < 32 || str_starts_with($secret, 'replace-'))) {
        throw new RuntimeException('Configurazione non sicura: APP_SECRET deve contenere almeno 32 caratteri.');
    }
}

function csrf_token(): string
{
    if (!isset($_SESSION['csrf'])) {
        $_SESSION['csrf'] = bin2hex(random_bytes(32));
    }
    return (string) $_SESSION['csrf'];
}

function verify_csrf(): void
{
    $given = $_POST['csrf'] ?? '';
    if (!is_string($given) || !hash_equals(csrf_token(), $given)) {
        http_response_code(419);
        throw new RuntimeException('Sessione scaduta. Ricarica la pagina e riprova.');
    }
}

function current_user(): ?array
{
    $id = $_SESSION['user_id'] ?? null;
    if (!is_int($id) && !ctype_digit((string) $id)) {
        return null;
    }
    $stmt = db()->prepare('SELECT u.id, u.email, u.display_name, u.role, u.is_active, u.approval_status, u.organization_id, u.organization_role, o.name AS organization_name, o.is_active AS organization_active FROM users u LEFT JOIN organizations o ON o.id = u.organization_id WHERE u.id = ?');
    $stmt->execute([(int) $id]);
    $user = $stmt->fetch();
    if (!$user || !(bool) $user['is_active'] || $user['approval_status'] !== 'approved' || ($user['organization_id'] !== null && !(bool) $user['organization_active'])) {
        unset($_SESSION['user_id']);
        return null;
    }
    return $user;
}

function tenant_for(array $user): array
{
    $organizationId = $user['organization_id'] === null ? null : (int) $user['organization_id'];
    $isSuperAdmin = $user['role'] === 'super_admin';
    $isOrganizationAdmin = $user['role'] === 'admin' && $organizationId !== null;
    $canManageAccounts = $isSuperAdmin || $isOrganizationAdmin;
    return [
        'organization_id' => $organizationId,
        'user_id' => (int) $user['id'],
        'name' => $organizationId === null ? 'Area personale' : (string) $user['organization_name'],
        'storage' => $organizationId === null ? 'users/' . (int) $user['id'] : 'organizations/' . $organizationId,
        'can_manage' => $organizationId === null || $isSuperAdmin || $isOrganizationAdmin,
        'can_manage_accounts' => $canManageAccounts,
        'is_super_admin' => $isSuperAdmin,
    ];
}

function is_account_admin(array $user): bool
{
    return $user['role'] === 'super_admin' || ($user['role'] === 'admin' && $user['organization_id'] !== null);
}

function require_user(): array
{
    $user = current_user();
    if ($user === null) {
        header('Location: /');
        exit;
    }
    return $user;
}

function site_setting(string $key): ?string
{
    if (!preg_match('/^[a-z0-9_.-]{1,64}$/', $key)) {
        throw new RuntimeException('Chiave di configurazione non valida.');
    }
    $stmt = db()->prepare('SELECT setting_value FROM site_settings WHERE setting_key = ?');
    $stmt->execute([$key]);
    $value = $stmt->fetchColumn();
    return is_string($value) && $value !== '' ? $value : null;
}

function save_site_setting(string $key, string $value, int $userId): void
{
    if (!preg_match('/^[a-z0-9_.-]{1,64}$/', $key) || mb_strlen($value) > 512) {
        throw new RuntimeException('Configurazione non valida.');
    }
    db()->prepare(
        'INSERT INTO site_settings (setting_key, setting_value, updated_by) VALUES (?, ?, ?) '
        . 'ON DUPLICATE KEY UPDATE setting_value = VALUES(setting_value), updated_by = VALUES(updated_by)'
    )->execute([$key, $value, $userId]);
}

function support_email(): ?string
{
    $email = trim((string) (site_setting('support_email') ?? env_value('SUPPORT_EMAIL', '')));
    return filter_var($email, FILTER_VALIDATE_EMAIL) ? $email : null;
}

function require_admin(): array
{
    $user = require_user();
    if (!is_account_admin($user)) {
        http_response_code(403);
        throw new RuntimeException('Operazione riservata agli amministratori.');
    }
    return $user;
}

function flash(string $type, string $message): void
{
    $_SESSION['flash'][] = ['type' => $type, 'message' => $message];
}

function take_flashes(): array
{
    $items = $_SESSION['flash'] ?? [];
    unset($_SESSION['flash']);
    return is_array($items) ? $items : [];
}

function redirect(string $page = 'dashboard'): never
{
    header('Location: /?page=' . rawurlencode($page));
    exit;
}

function audit(?int $userId, string $event, array $details = [], ?int $organizationId = null): void
{
    $secret = (string) env_value('APP_SECRET', 'change-me');
    $ip = $_SERVER['REMOTE_ADDR'] ?? '';
    $ipHash = $ip === '' ? null : hash_hmac('sha256', $ip, $secret);
    if ($organizationId === null && $userId !== null) {
        $lookup = db()->prepare('SELECT organization_id FROM users WHERE id = ?');
        $lookup->execute([$userId]);
        $value = $lookup->fetchColumn();
        $organizationId = $value === false || $value === null ? null : (int) $value;
    }
    $stmt = db()->prepare('INSERT INTO audit_events (user_id, organization_id, event_type, details_json, ip_hash) VALUES (?, ?, ?, ?, ?)');
    $stmt->execute([$userId, $organizationId, $event, $details === [] ? null : json_encode($details, JSON_THROW_ON_ERROR), $ipHash]);
}

function validate_pdf_upload(array $file, string $label): array
{
    if (($file['error'] ?? UPLOAD_ERR_NO_FILE) !== UPLOAD_ERR_OK) {
        throw new RuntimeException("Caricamento {$label} non riuscito.");
    }
    $size = (int) ($file['size'] ?? 0);
    $max = max(1, (int) env_value('MAX_UPLOAD_MB', '200')) * 1024 * 1024;
    if ($size < 5 || $size > $max) {
        throw new RuntimeException("{$label}: dimensione non consentita.");
    }
    $tmp = (string) ($file['tmp_name'] ?? '');
    if (!is_uploaded_file($tmp)) {
        throw new RuntimeException("{$label}: file temporaneo non valido.");
    }
    $handle = fopen($tmp, 'rb');
    $magic = $handle === false ? '' : (string) fread($handle, 5);
    if (is_resource($handle)) {
        fclose($handle);
    }
    if ($magic !== '%PDF-') {
        throw new RuntimeException("{$label}: seleziona un file PDF valido.");
    }
    $original = basename((string) ($file['name'] ?? 'cover.pdf'));
    return [$tmp, mb_substr($original, 0, 255)];
}

function random_id(): string
{
    $hex = bin2hex(random_bytes(16));
    return substr($hex, 0, 8) . '-' . substr($hex, 8, 4) . '-4' . substr($hex, 13, 3) . '-a' . substr($hex, 17, 3) . '-' . substr($hex, 20, 12);
}

function cover_file_path(array $tenant, string $storageName): string
{
    if (!preg_match('/^[0-9a-f-]{36}$/', $storageName)) {
        throw new RuntimeException('Identificativo cover non valido.');
    }
    return storage_path($tenant['storage'] . "/covers/{$storageName}.pdf");
}

function remove_tree(string $directory): void
{
    if (!is_dir($directory)) {
        return;
    }
    $items = scandir($directory);
    if ($items === false) {
        return;
    }
    foreach ($items as $item) {
        if ($item === '.' || $item === '..') {
            continue;
        }
        $path = $directory . DIRECTORY_SEPARATOR . $item;
        if (is_dir($path)) {
            remove_tree($path);
        } else {
            @unlink($path);
        }
    }
    @rmdir($directory);
}

load_environment_file();
validate_configuration();
security_headers();
start_secure_session();
