<?php
declare(strict_types=1);

require __DIR__ . '/../src/bootstrap.php';
require __DIR__ . '/../src/covers.php';
require __DIR__ . '/../src/documents.php';

const API_MAX_JSON = 300_000_000;
const API_MAX_AI_TEXT = 500_000;

header('Cache-Control: private, no-store');

function api_json(array $value, int $status = 200): never
{
    http_response_code($status);
    header('Content-Type: application/json; charset=utf-8');
    echo json_encode($value, JSON_THROW_ON_ERROR | JSON_UNESCAPED_UNICODE);
    exit;
}

function api_body(): array
{
    $length = (int) ($_SERVER['CONTENT_LENGTH'] ?? 0);
    if ($length < 0 || $length > API_MAX_JSON) {
        throw new RuntimeException('Richiesta troppo grande.');
    }
    $decoded = json_decode((string) file_get_contents('php://input'), true, 64, JSON_THROW_ON_ERROR);
    if (!is_array($decoded)) {
        throw new RuntimeException('Richiesta JSON non valida.');
    }
    return $decoded;
}

function api_bearer(): ?string
{
    $header = (string) ($_SERVER['HTTP_AUTHORIZATION'] ?? $_SERVER['REDIRECT_HTTP_AUTHORIZATION'] ?? '');
    if ($header === '' && function_exists('getallheaders')) {
        $headers = getallheaders();
        if (is_array($headers)) {
            $header = (string) ($headers['Authorization'] ?? $headers['authorization'] ?? '');
        }
    }
    return preg_match('/^Bearer ([A-Za-z0-9_-]{40,100})$/', $header, $match) === 1 ? $match[1] : null;
}

function api_user(): ?array
{
    $token = api_bearer();
    if ($token === null) {
        return current_user();
    }
    $stmt = db()->prepare(
        'SELECT u.id, u.email, u.display_name, u.role, u.is_active, u.approval_status, u.organization_id, u.organization_role, '
        . 'o.name AS organization_name, o.is_active AS organization_active '
        . 'FROM api_tokens t JOIN users u ON u.id = t.user_id '
        . 'LEFT JOIN organizations o ON o.id = u.organization_id '
        . 'WHERE t.token_hash = ? AND t.expires_at > CURRENT_TIMESTAMP'
    );
    $hash = hash('sha256', $token);
    $stmt->execute([$hash]);
    $user = $stmt->fetch();
    if (!$user || !(bool) $user['is_active'] || $user['approval_status'] !== 'approved' || ($user['organization_id'] !== null && !(bool) $user['organization_active'])) {
        return null;
    }
    db()->prepare('UPDATE api_tokens SET last_used_at = CURRENT_TIMESTAMP WHERE token_hash = ?')->execute([$hash]);
    return $user;
}

function api_require_user(): array
{
    $user = api_user();
    if ($user === null) {
        throw new RuntimeException('Autenticazione richiesta.', 401);
    }
    return $user;
}

function api_authorize_write(array $body): void
{
    if (api_bearer() !== null) {
        return;
    }
    $_POST['csrf'] = is_string($body['csrf'] ?? null) ? $body['csrf'] : '';
    verify_csrf();
}

function api_secret_key(): string
{
    return hash('sha256', (string) env_value('APP_SECRET', ''), true);
}

function api_encrypt(string $plain): string
{
    $nonce = random_bytes(12);
    $tag = '';
    $cipher = openssl_encrypt($plain, 'aes-256-gcm', api_secret_key(), OPENSSL_RAW_DATA, $nonce, $tag);
    if (!is_string($cipher) || strlen($tag) !== 16) {
        throw new RuntimeException('Impossibile proteggere la credenziale.');
    }
    return base64_encode($nonce . $tag . $cipher);
}

function api_decrypt(?string $encoded): string
{
    if ($encoded === null || $encoded === '') {
        return '';
    }
    $raw = base64_decode($encoded, true);
    if (!is_string($raw) || strlen($raw) < 29) {
        throw new RuntimeException('Credenziale salvata non valida.');
    }
    $plain = openssl_decrypt(substr($raw, 28), 'aes-256-gcm', api_secret_key(), OPENSSL_RAW_DATA, substr($raw, 0, 12), substr($raw, 12, 16));
    if (!is_string($plain)) {
        throw new RuntimeException('Impossibile leggere la credenziale salvata.');
    }
    return $plain;
}

function api_clean_ids(mixed $value, string $error): array
{
    if (!is_array($value) || $value === [] || count($value) > 100) throw new RuntimeException($error);
    $ids = [];
    foreach ($value as $id) {
        if (!is_int($id) && !ctype_digit((string) $id)) throw new RuntimeException($error);
        $clean = (int) $id;
        if ($clean < 1 || in_array($clean, $ids, true)) throw new RuntimeException($error);
        $ids[] = $clean;
    }
    return $ids;
}

function api_remove_cover_files(string $storage, array $sets): void
{
    $tenant = ['storage' => $storage];
    foreach ($sets as $set) {
        foreach (['front_storage_name', 'back_storage_name'] as $key) {
            $name = $set[$key] ?? null;
            if (is_string($name) && preg_match('/^[0-9a-f-]{36}$/', $name) === 1) {
                @unlink(cover_file_path($tenant, $name));
                @unlink(cover_preview_path($tenant, $name));
            }
        }
    }
}

function api_public_ai(array $config, ?int $ownerUserId = null, ?string $ownerName = null): array
{
    return [
        'provider' => (string) ($config['provider'] ?? 'openai'),
        'auth_mode' => (string) ($config['auth_mode'] ?? 'api_key'),
        'base_url' => (string) ($config['base_url'] ?? ''),
        'model' => (string) ($config['model'] ?? ''),
        'has_secret' => is_string($config['secret_cipher'] ?? null) && $config['secret_cipher'] !== '',
        'owner_user_id' => $ownerUserId,
        'owner_name' => $ownerName,
    ];
}

function api_snapshot(array $user): array
{
    $tenant = tenant_for($user);
    $kits = array_map(static fn(array $kit): array => [
        'id' => (int) $kit['id'],
        'name' => (string) $kit['name'],
        'front_name' => $kit['front_original_name'],
        'back_name' => $kit['back_original_name'],
        'is_default' => (bool) $kit['is_default'],
        'front_preview' => cover_preview_base64($tenant, $kit['front_storage_name']),
        'back_preview' => cover_preview_base64($tenant, $kit['back_storage_name']),
    ], user_cover_sets($tenant));

    $users = [];
    $organizations = [];
    if ($tenant['can_manage_accounts']) {
        if ($user['role'] === 'super_admin') {
            $users = db()->query('SELECT u.id, u.email, u.display_name, u.role, u.organization_id, u.organization_role, u.is_active, u.approval_status, o.name AS organization_name FROM users u LEFT JOIN organizations o ON o.id = u.organization_id ORDER BY u.approval_status, u.display_name')->fetchAll();
            $organizations = db()->query('SELECT o.id, o.name, o.slug, o.is_active, COUNT(u.id) AS member_count FROM organizations o LEFT JOIN users u ON u.organization_id = o.id GROUP BY o.id, o.name, o.slug, o.is_active ORDER BY o.name')->fetchAll();
        } elseif ($tenant['organization_id'] !== null) {
            $stmt = db()->prepare('SELECT u.id, u.email, u.display_name, u.role, u.organization_id, u.organization_role, u.is_active, u.approval_status, o.name AS organization_name FROM users u JOIN organizations o ON o.id = u.organization_id WHERE u.organization_id = ? ORDER BY u.approval_status, u.display_name');
            $stmt->execute([$tenant['organization_id']]);
            $users = $stmt->fetchAll();
            $stmt = db()->prepare('SELECT o.id, o.name, o.slug, o.is_active, COUNT(u.id) AS member_count FROM organizations o LEFT JOIN users u ON u.organization_id = o.id WHERE o.id = ? GROUP BY o.id, o.name, o.slug, o.is_active');
            $stmt->execute([$tenant['organization_id']]);
            $organizations = $stmt->fetchAll();
        }
    }
    $users = array_map(static fn(array $item): array => [
        'id' => (int) $item['id'], 'email' => (string) $item['email'], 'display_name' => (string) $item['display_name'],
        'role' => (string) $item['role'], 'organization_id' => $item['organization_id'] === null ? null : (int) $item['organization_id'],
        'organization_name' => $item['organization_name'], 'organization_role' => $item['organization_role'], 'is_active' => (bool) $item['is_active'],
        'approval_status' => (string) $item['approval_status'],
    ], $users);
    $organizations = array_map(static fn(array $item): array => [
        'id' => (int) $item['id'], 'name' => (string) $item['name'], 'slug' => (string) $item['slug'],
        'is_active' => (bool) $item['is_active'], 'member_count' => (int) $item['member_count'],
    ], $organizations);

    $stmt = db()->prepare('SELECT provider, auth_mode, base_url, model, secret_cipher FROM ai_provider_settings WHERE user_id = ?');
    $stmt->execute([(int) $user['id']]);
    $personalAi = $stmt->fetch() ?: ['provider' => 'openai', 'auth_mode' => 'api_key', 'base_url' => '', 'model' => '', 'secret_cipher' => null];
    $sharedAi = null;
    if ($user['organization_id'] !== null) {
        $stmt = db()->prepare(
            'SELECT s.owner_user_id, owner.display_name AS owner_name, a.provider, a.auth_mode, a.base_url, a.model, a.secret_cipher '
            . 'FROM ai_provider_shares s JOIN users owner ON owner.id = s.owner_user_id '
            . 'JOIN ai_provider_settings a ON a.user_id = s.owner_user_id '
            . 'WHERE s.target_user_id = ? AND owner.organization_id = ? AND owner.role = \'admin\' AND owner.is_active = 1 AND owner.approval_status = \'approved\' LIMIT 1'
        );
        $stmt->execute([(int) $user['id'], (int) $user['organization_id']]);
        $sharedAi = $stmt->fetch() ?: null;
    }
    $stmt = db()->prepare('SELECT ai_source FROM user_preferences WHERE user_id = ?');
    $stmt->execute([(int) $user['id']]);
    $preferredSource = (string) ($stmt->fetchColumn() ?: 'personal');
    $aiSource = $preferredSource === 'organization' && is_array($sharedAi) ? 'organization' : 'personal';
    $sharedUserIds = [];
    if ($user['role'] === 'admin' && $user['organization_id'] !== null) {
        $stmt = db()->prepare('SELECT target_user_id FROM ai_provider_shares WHERE owner_user_id = ? ORDER BY target_user_id');
        $stmt->execute([(int) $user['id']]);
        $sharedUserIds = array_map('intval', $stmt->fetchAll(PDO::FETCH_COLUMN));
    }
    $personalPublic = api_public_ai($personalAi);
    $sharedPublic = is_array($sharedAi)
        ? api_public_ai($sharedAi, (int) $sharedAi['owner_user_id'], (string) $sharedAi['owner_name'])
        : null;

    return [
        'account' => [
            'id' => (int) $user['id'], 'email' => (string) $user['email'], 'display_name' => (string) $user['display_name'],
            'role' => (string) $user['role'], 'organization_id' => $user['organization_id'] === null ? null : (int) $user['organization_id'],
            'organization_name' => $user['organization_name'], 'organization_role' => $user['organization_role'], 'can_manage' => (bool) $tenant['can_manage'],
            'can_manage_accounts' => (bool) $tenant['can_manage_accounts'],
        ],
        'users' => $users,
        'organizations' => $organizations,
        'cover_kits' => $kits,
        'ai' => $aiSource === 'organization' && $sharedPublic !== null ? $sharedPublic : $personalPublic,
        'personal_ai' => $personalPublic,
        'shared_ai' => $sharedPublic,
        'ai_source' => $aiSource,
        'ai_shared_user_ids' => $sharedUserIds,
        'share_users' => document_share_members($user),
        'notifications' => document_notifications($user),
        'shared_documents' => received_shared_documents($user),
        'csrf' => api_bearer() === null ? csrf_token() : null,
    ];
}

function api_decode_pdf(mixed $encoded, string $label): ?string
{
    if ($encoded === null) {
        return null;
    }
    if (!is_string($encoded)) {
        throw new RuntimeException("{$label}: dati non validi.");
    }
    $bytes = base64_decode($encoded, true);
    $max = max(1, (int) env_value('MAX_UPLOAD_MB', '200')) * 1024 * 1024;
    if (!is_string($bytes) || strlen($bytes) < 5 || strlen($bytes) > $max || !str_starts_with($bytes, '%PDF-')) {
        throw new RuntimeException("{$label}: seleziona un PDF valido.");
    }
    return $bytes;
}

function api_save_cover(array $tenant, array $body): void
{
    if (!$tenant['can_manage']) {
        throw new RuntimeException('Operazione riservata agli amministratori dell’organizzazione.');
    }
    $name = trim((string) ($body['name'] ?? ''));
    if ($name === '' || mb_strlen($name) > 120) {
        throw new RuntimeException('Inserisci un nome di massimo 120 caratteri.');
    }
    $front = api_decode_pdf($body['front'] ?? null, 'Cover frontale');
    $back = api_decode_pdf($body['back'] ?? null, 'Cover posteriore');
    if ($front === null && $back === null) {
        throw new RuntimeException('Carica almeno una cover.');
    }
    $directory = storage_path($tenant['storage'] . '/covers');
    ensure_directory($directory);
    $frontId = $front === null ? null : random_id();
    $backId = $back === null ? null : random_id();
    $saved = [];
    try {
        foreach ([[$front, $frontId], [$back, $backId]] as [$bytes, $id]) {
            if (is_string($bytes) && is_string($id)) {
                $path = cover_file_path($tenant, $id);
                if (file_put_contents($path, $bytes, LOCK_EX) !== strlen($bytes)) {
                    throw new RuntimeException('Impossibile salvare una cover.');
                }
                $saved[] = $path;
            }
        }
        $pdo = db();
        $pdo->beginTransaction();
        $makeDefault = (bool) ($body['make_default'] ?? false) || count(user_cover_sets($tenant)) === 0;
        if ($makeDefault) clear_default_cover($pdo, $tenant);
        $stmt = $pdo->prepare('INSERT INTO cover_sets (user_id, organization_id, name, front_original_name, front_storage_name, back_original_name, back_storage_name, is_default) VALUES (?, ?, ?, ?, ?, ?, ?, ?)');
        $stmt->execute([
            $tenant['user_id'], $tenant['organization_id'], $name,
            $front === null ? null : mb_substr(basename((string) ($body['front_name'] ?? 'front.pdf')), 0, 255), $frontId,
            $back === null ? null : mb_substr(basename((string) ($body['back_name'] ?? 'back.pdf')), 0, 255), $backId,
            $makeDefault ? 1 : 0,
        ]);
        $pdo->commit();
    } catch (Throwable $error) {
        if (db()->inTransaction()) db()->rollBack();
        foreach ($saved as $path) @unlink($path);
        throw $error;
    }
}

function api_http_json(string $method, string $url, array $headers = [], ?array $payload = null): array
{
    if (!filter_var($url, FILTER_VALIDATE_URL) || (!str_starts_with($url, 'https://') && env_value('APP_ENV', 'production') === 'production')) {
        throw new RuntimeException('URL del provider non consentito.');
    }
    $host = parse_url($url, PHP_URL_HOST);
    if (!is_string($host) || $host === '') throw new RuntimeException('Host del provider non valido.');
    if (env_value('APP_ENV', 'production') === 'production') {
        $addresses = gethostbynamel($host);
        if (!is_array($addresses) || $addresses === []) throw new RuntimeException('Host del provider non risolvibile.');
        foreach ($addresses as $address) {
            if (filter_var($address, FILTER_VALIDATE_IP, FILTER_FLAG_NO_PRIV_RANGE | FILTER_FLAG_NO_RES_RANGE) === false) {
                throw new RuntimeException('Il provider non può usare un indirizzo di rete privato o riservato.');
            }
        }
    }
    $curl = curl_init($url);
    if ($curl === false) throw new RuntimeException('Client HTTP non disponibile.');
    $headers[] = 'Accept: application/json';
    $options = [
        CURLOPT_RETURNTRANSFER => true, CURLOPT_FOLLOWLOCATION => false, CURLOPT_CONNECTTIMEOUT => 10, CURLOPT_TIMEOUT => 90,
        CURLOPT_CUSTOMREQUEST => $method, CURLOPT_HTTPHEADER => $headers, CURLOPT_PROTOCOLS => CURLPROTO_HTTP | CURLPROTO_HTTPS,
    ];
    if ($payload !== null) {
        $options[CURLOPT_POSTFIELDS] = json_encode($payload, JSON_THROW_ON_ERROR | JSON_UNESCAPED_UNICODE);
        $options[CURLOPT_HTTPHEADER][] = 'Content-Type: application/json';
    }
    curl_setopt_array($curl, $options);
    $raw = curl_exec($curl);
    $status = (int) curl_getinfo($curl, CURLINFO_RESPONSE_CODE);
    $error = curl_error($curl);
    curl_close($curl);
    if (!is_string($raw)) throw new RuntimeException('Provider non raggiungibile: ' . $error);
    if (strlen($raw) > 4 * 1024 * 1024) throw new RuntimeException('Risposta del provider troppo grande.');
    $decoded = json_decode($raw, true, 128, JSON_THROW_ON_ERROR);
    if ($status < 200 || $status >= 300) {
        $message = is_array($decoded) ? ($decoded['error']['message'] ?? $decoded['error'] ?? "HTTP {$status}") : "HTTP {$status}";
        throw new RuntimeException('Provider: ' . (is_string($message) ? $message : "HTTP {$status}"));
    }
    if (!is_array($decoded)) throw new RuntimeException('Risposta del provider non valida.');
    return $decoded;
}

function api_refresh_chatgpt_token(int $userId, array $bundle): array
{
    $refreshToken = $bundle['refresh_token'] ?? null;
    $clientId = (string) env_value('OPENAI_CLIENT_ID', '');
    if (!is_string($refreshToken) || $refreshToken === '' || $clientId === '') {
        throw new RuntimeException('Accesso ChatGPT scaduto. Collegalo di nuovo dalle Preferenze.');
    }
    $curl = curl_init('https://auth.openai.com/api/accounts/oauth/token');
    if ($curl === false) throw new RuntimeException('Client OAuth non disponibile.');
    curl_setopt_array($curl, [
        CURLOPT_RETURNTRANSFER => true,
        CURLOPT_POST => true,
        CURLOPT_CONNECTTIMEOUT => 10,
        CURLOPT_TIMEOUT => 45,
        CURLOPT_POSTFIELDS => http_build_query([
            'grant_type' => 'refresh_token',
            'client_id' => $clientId,
            'refresh_token' => $refreshToken,
        ]),
    ]);
    $raw = curl_exec($curl);
    $status = (int) curl_getinfo($curl, CURLINFO_RESPONSE_CODE);
    curl_close($curl);
    $tokens = is_string($raw) ? json_decode($raw, true) : null;
    if ($status < 200 || $status >= 300 || !is_array($tokens) || !is_string($tokens['access_token'] ?? null)) {
        throw new RuntimeException('Rinnovo ChatGPT non riuscito. Collegalo di nuovo dalle Preferenze.');
    }
    $next = [
        'access_token' => $tokens['access_token'],
        'refresh_token' => is_string($tokens['refresh_token'] ?? null) ? $tokens['refresh_token'] : $refreshToken,
        'expires_at' => time() + max(60, (int) ($tokens['expires_in'] ?? 3600)),
    ];
    db()->prepare('UPDATE ai_provider_settings SET secret_cipher = ? WHERE user_id = ?')
        ->execute([api_encrypt(json_encode($next, JSON_THROW_ON_ERROR)), $userId]);
    return $next;
}

function api_ai_config(array $user, ?string $requestedSource = null): array
{
    $userId = (int) $user['id'];
    $sourceWasExplicit = $requestedSource !== null;
    $source = $requestedSource;
    if ($source === null) {
        $stmt = db()->prepare('SELECT ai_source FROM user_preferences WHERE user_id = ?');
        $stmt->execute([$userId]);
        $source = (string) ($stmt->fetchColumn() ?: 'personal');
    }
    $ownerUserId = $userId;
    if ($source === 'organization') {
        if ($user['organization_id'] === null && $sourceWasExplicit) throw new RuntimeException('Nessuna AI dellâ€™organizzazione disponibile.');
        if ($user['organization_id'] === null) $source = 'personal';
    }
    if ($source === 'organization') {
        $stmt = db()->prepare(
            'SELECT s.owner_user_id FROM ai_provider_shares s JOIN users owner ON owner.id = s.owner_user_id '
            . 'WHERE s.target_user_id = ? AND owner.organization_id = ? AND owner.role = \'admin\' AND owner.is_active = 1 AND owner.approval_status = \'approved\' LIMIT 1'
        );
        $stmt->execute([$userId, (int) $user['organization_id']]);
        $ownerUserId = (int) ($stmt->fetchColumn() ?: 0);
        if ($ownerUserId <= 0 && $sourceWasExplicit) throw new RuntimeException('Lâ€™AI dellâ€™organizzazione non Ã¨ stata condivisa con questo account.');
        if ($ownerUserId <= 0) $ownerUserId = $userId;
    }
    $stmt = db()->prepare('SELECT provider, auth_mode, base_url, model, secret_cipher FROM ai_provider_settings WHERE user_id = ?');
    $stmt->execute([$ownerUserId]);
    $config = $stmt->fetch();
    if (!$config) throw new RuntimeException('Configura prima un provider AI.');
    $secret = api_decrypt(is_string($config['secret_cipher']) ? $config['secret_cipher'] : null);
    if ($config['provider'] === 'openai' && $config['auth_mode'] === 'chatgpt_oauth') {
        $bundle = json_decode($secret, true);
        if (is_array($bundle) && is_string($bundle['access_token'] ?? null)) {
            if ((int) ($bundle['expires_at'] ?? 0) <= time() + 60) $bundle = api_refresh_chatgpt_token($ownerUserId, $bundle);
            $secret = (string) $bundle['access_token'];
        }
    }
    $config['secret'] = $secret;
    if ($config['secret'] === '' && $config['provider'] !== 'compatible') throw new RuntimeException('Credenziale del provider mancante.');
    $config['owner_user_id'] = $ownerUserId;
    return $config;
}

function api_ai_models(array $config): array
{
    $provider = (string) $config['provider'];
    if ($provider === 'gemini') {
        $json = api_http_json('GET', 'https://generativelanguage.googleapis.com/v1beta/models?key=' . rawurlencode((string) $config['secret']));
        $models = array_map(static fn(array $model): string => preg_replace('#^models/#', '', (string) ($model['name'] ?? '')), $json['models'] ?? []);
    } else {
        $base = match ($provider) {
            'openai' => 'https://api.openai.com/v1',
            'openrouter' => 'https://openrouter.ai/api/v1',
            default => rtrim((string) $config['base_url'], '/'),
        };
        $headers = $config['secret'] === '' ? [] : ['Authorization: Bearer ' . $config['secret']];
        $json = api_http_json('GET', $base . '/models', $headers);
        $models = array_map(static fn(array $model): string => (string) ($model['id'] ?? ''), $json['data'] ?? []);
    }
    $models = array_values(array_unique(array_filter($models, static fn(string $model): bool => $model !== '' && strlen($model) <= 160)));
    sort($models, SORT_NATURAL | SORT_FLAG_CASE);
    return array_slice($models, 0, 500);
}

function api_ai_run(array $config, array $body): string
{
    $text = (string) ($body['text'] ?? '');
    if ($text === '' || mb_strlen($text) > API_MAX_AI_TEXT) throw new RuntimeException('Testo del documento vuoto o troppo grande.');
    $task = (string) ($body['task'] ?? 'summary');
    if (!in_array($task, ['summary', 'ask', 'translate'], true)) throw new RuntimeException('Operazione AI non valida.');
    $question = trim((string) ($body['question'] ?? ''));
    $language = trim((string) ($body['language'] ?? ''));
    if ($task === 'ask' && ($question === '' || mb_strlen($question) > 4000)) throw new RuntimeException('Inserisci una domanda valida.');
    if ($task === 'translate' && ($language === '' || mb_strlen($language) > 80)) throw new RuntimeException('Inserisci una lingua valida.');
    $instruction = match ($task) {
        'ask' => 'Answer the question using only the document. Cite the relevant [Page N] markers. If the answer is absent, say so. Question: ' . $question,
        'translate' => 'Translate the document faithfully into ' . $language . '. Preserve headings and lists.',
        default => 'Summarize the document concisely. Preserve key facts, dates, people, decisions and action items.',
    };
    $provider = (string) $config['provider'];
    $model = (string) $config['model'];
    if ($model === '') throw new RuntimeException('Seleziona un modello AI.');
    if ($provider === 'gemini') {
        $url = 'https://generativelanguage.googleapis.com/v1beta/models/' . rawurlencode($model) . ':generateContent?key=' . rawurlencode((string) $config['secret']);
        $json = api_http_json('POST', $url, [], ['contents' => [['parts' => [['text' => $instruction . "\n\nDOCUMENT:\n" . $text]]]]]);
        $answer = $json['candidates'][0]['content']['parts'][0]['text'] ?? null;
    } elseif ($provider === 'openai' && $config['auth_mode'] === 'chatgpt_oauth') {
        $json = api_http_json('POST', 'https://api.openai.com/v1/responses', ['Authorization: Bearer ' . $config['secret']], [
            'model' => $model, 'input' => $instruction . "\n\nDOCUMENT:\n" . $text,
        ]);
        $answer = $json['output_text'] ?? ($json['output'][0]['content'][0]['text'] ?? null);
    } else {
        $base = match ($provider) {
            'openai' => 'https://api.openai.com/v1', 'openrouter' => 'https://openrouter.ai/api/v1', default => rtrim((string) $config['base_url'], '/'),
        };
        $headers = $config['secret'] === '' ? [] : ['Authorization: Bearer ' . $config['secret']];
        $json = api_http_json('POST', $base . '/chat/completions', $headers, [
            'model' => $model,
            'messages' => [['role' => 'system', 'content' => $instruction], ['role' => 'user', 'content' => $text]],
        ]);
        $answer = $json['choices'][0]['message']['content'] ?? null;
    }
    if (!is_string($answer) || trim($answer) === '') throw new RuntimeException('Il provider non ha restituito testo.');
    return mb_substr($answer, 0, 200_000);
}

try {
    $action = (string) ($_GET['action'] ?? 'snapshot');
    if ($action === 'chatgpt-callback') {
        $state = (string) ($_GET['state'] ?? '');
        $code = (string) ($_GET['code'] ?? '');
        $stmt = db()->prepare('SELECT user_id, verifier_cipher FROM oauth_transactions WHERE state_hash = ? AND expires_at > CURRENT_TIMESTAMP');
        $stmt->execute([hash('sha256', $state)]);
        $transaction = $stmt->fetch();
        if (!$transaction || $code === '') throw new RuntimeException('Autorizzazione ChatGPT scaduta o non valida.');
        db()->prepare('DELETE FROM oauth_transactions WHERE state_hash = ?')->execute([hash('sha256', $state)]);
        $clientId = (string) env_value('OPENAI_CLIENT_ID', '');
        $redirect = rtrim((string) env_value('APP_URL', ''), '/') . '/app-api.php?action=chatgpt-callback';
        $curl = curl_init('https://auth.openai.com/api/accounts/oauth/token');
        if ($curl === false) throw new RuntimeException('Client OAuth non disponibile.');
        curl_setopt_array($curl, [CURLOPT_RETURNTRANSFER => true, CURLOPT_POST => true, CURLOPT_POSTFIELDS => http_build_query([
            'grant_type' => 'authorization_code', 'client_id' => $clientId, 'code' => $code,
            'code_verifier' => api_decrypt((string) $transaction['verifier_cipher']), 'redirect_uri' => $redirect,
            'resource' => 'https://api.openai.com/v1',
        ]), CURLOPT_TIMEOUT => 30]);
        $raw = curl_exec($curl);
        $status = (int) curl_getinfo($curl, CURLINFO_RESPONSE_CODE);
        curl_close($curl);
        $tokens = is_string($raw) ? json_decode($raw, true) : null;
        if ($status < 200 || $status >= 300 || !is_array($tokens) || !is_string($tokens['access_token'] ?? null)) {
            throw new RuntimeException('ChatGPT non ha completato l’autorizzazione.');
        }
        $bundle = [
            'access_token' => $tokens['access_token'],
            'refresh_token' => is_string($tokens['refresh_token'] ?? null) ? $tokens['refresh_token'] : null,
            'expires_at' => time() + max(60, (int) ($tokens['expires_in'] ?? 3600)),
        ];
        db()->prepare("INSERT INTO ai_provider_settings (user_id, provider, auth_mode, secret_cipher) VALUES (?, 'openai', 'chatgpt_oauth', ?) ON DUPLICATE KEY UPDATE provider='openai', auth_mode='chatgpt_oauth', secret_cipher=VALUES(secret_cipher)")
            ->execute([(int) $transaction['user_id'], api_encrypt(json_encode($bundle, JSON_THROW_ON_ERROR))]);
        header('Location: /editor/?chatgpt=connected');
        exit;
    }

    $body = $_SERVER['REQUEST_METHOD'] === 'POST' ? api_body() : [];
    if ($action === 'login') {
        $email = mb_strtolower(trim((string) ($body['email'] ?? '')));
        $password = (string) ($body['password'] ?? '');
        $stmt = db()->prepare('SELECT u.*, o.name AS organization_name, o.is_active AS organization_active FROM users u LEFT JOIN organizations o ON o.id = u.organization_id WHERE u.email = ? LIMIT 1');
        $stmt->execute([$email]);
        $user = $stmt->fetch();
        if (!$user || !(bool) $user['is_active'] || ($user['approval_status'] ?? 'approved') !== 'approved' || ($user['organization_id'] !== null && !(bool) $user['organization_active']) || !password_verify($password, (string) $user['password_hash'])) {
            audit($user ? (int) $user['id'] : null, 'api_login_failed');
            throw new RuntimeException('Credenziali non valide.', 401);
        }
        $token = rtrim(strtr(base64_encode(random_bytes(48)), '+/', '-_'), '=');
        db()->prepare('INSERT INTO api_tokens (token_hash, user_id, expires_at) VALUES (?, ?, DATE_ADD(CURRENT_TIMESTAMP, INTERVAL 30 DAY))')->execute([hash('sha256', $token), (int) $user['id']]);
        audit((int) $user['id'], 'api_login_succeeded');
        api_json(['token' => $token, 'snapshot' => api_snapshot($user)]);
    }

    $user = api_require_user();
    $tenant = tenant_for($user);
    if ($action === 'snapshot') api_json(['snapshot' => api_snapshot($user)]);
    if ($action === 'document-open') {
        if ($_SERVER['REQUEST_METHOD'] !== 'GET') throw new RuntimeException('Metodo non consentito.', 405);
        $document = shared_document_for_download($user, (int) ($_GET['id'] ?? 0));
        header('Content-Type: application/pdf');
        header('Content-Disposition: attachment; filename="documento.pdf"; filename*=UTF-8\'\'' . rawurlencode($document['name']));
        header('Content-Length: ' . $document['size']);
        header('X-Content-Type-Options: nosniff');
        $sent = readfile($document['path']);
        if ($sent === false) throw new RuntimeException('Impossibile leggere il documento condiviso.');
        exit;
    }
    if ($_SERVER['REQUEST_METHOD'] !== 'POST') throw new RuntimeException('Metodo non consentito.', 405);
    api_authorize_write($body);

    if ($action === 'document-share') {
        $bytes = api_decode_pdf($body['document'] ?? null, 'Documento');
        if (!is_string($bytes)) throw new RuntimeException('Documento mancante.');
        $recipientIds = $body['recipient_ids'] ?? null;
        if (!is_array($recipientIds)) throw new RuntimeException('Selezione destinatari non valida.');
        create_shared_document($user, (string) ($body['name'] ?? 'documento.pdf'), $bytes, $recipientIds);
        audit((int) $user['id'], 'document_shared_in_app', ['recipient_count' => count($recipientIds)]);
        api_json(['message' => 'Documento condiviso.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'notification-read') {
        mark_notification_read($user, (int) ($body['id'] ?? 0));
        api_json(['message' => 'Notifica letta.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'notifications-read-all') {
        mark_all_notifications_read($user);
        api_json(['message' => 'Notifiche segnate come lette.', 'snapshot' => api_snapshot($user)]);
    }

    if ($action === 'cover-load') {
        $set = cover_set_for_tenant($tenant, (int) ($body['id'] ?? 0));
        if (!$set) throw new RuntimeException('Kit cover non trovato.');
        $read = static function (?string $storage) use ($tenant): ?string {
            if ($storage === null || $storage === '') return null;
            $bytes = file_get_contents(cover_file_path($tenant, $storage));
            if (!is_string($bytes)) throw new RuntimeException('File cover non disponibile.');
            return base64_encode($bytes);
        };
        api_json(['id' => (int) $set['id'], 'front_name' => $set['front_original_name'], 'front' => $read($set['front_storage_name']), 'back_name' => $set['back_original_name'], 'back' => $read($set['back_storage_name'])]);
    }
    if ($action === 'cover-save') {
        api_save_cover($tenant, $body); audit((int) $user['id'], 'cover_set_created_in_app');
        api_json(['message' => 'Kit cover salvato.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'cover-default') {
        set_default_cover($tenant, (int) ($body['id'] ?? 0));
        api_json(['message' => 'Kit predefinito aggiornato.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'cover-delete') {
        delete_cover_set($tenant, (int) ($body['id'] ?? 0));
        api_json(['message' => 'Kit cover eliminato.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'cover-delete-many') {
        $ids = $body['ids'] ?? null;
        if (!is_array($ids) || $ids === [] || count($ids) > 100) throw new RuntimeException('Selezione kit non valida.');
        $clean = [];
        foreach ($ids as $id) {
            if (!is_int($id) && !ctype_digit((string) $id)) throw new RuntimeException('Selezione kit non valida.');
            $value = (int) $id;
            if ($value < 1 || in_array($value, $clean, true)) throw new RuntimeException('Selezione kit non valida.');
            $clean[] = $value;
        }
        foreach ($clean as $id) delete_cover_set($tenant, $id);
        audit((int) $user['id'], 'cover_sets_deleted_in_app', ['count' => count($clean)]);
        api_json(['message' => 'Kit cover selezionati eliminati.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'user-create') {
        if (!$tenant['can_manage_accounts']) throw new RuntimeException('Operazione riservata agli amministratori.', 403);
        $organizationId = isset($body['organization_id']) ? (int) $body['organization_id'] : null;
        $isSuperAdmin = $user['role'] === 'super_admin';
        if (!$isSuperAdmin) $organizationId = $tenant['organization_id'];
        $email = mb_strtolower(trim((string) ($body['email'] ?? '')));
        $name = trim((string) ($body['display_name'] ?? ''));
        $password = (string) ($body['password'] ?? '');
        if (!filter_var($email, FILTER_VALIDATE_EMAIL) || $name === '' || strlen($password) < 8) throw new RuntimeException('Dati utente non validi.');
        $requestedRole = (string) ($body['role'] ?? 'user');
        $allowedRoles = $isSuperAdmin ? ['super_admin', 'admin', 'user'] : ['admin', 'user'];
        $role = in_array($requestedRole, $allowedRoles, true) ? $requestedRole : 'user';
        if ($role === 'super_admin') {
            $organizationId = null;
        } elseif ($role === 'admin' && $organizationId === null) {
            throw new RuntimeException('Un amministratore deve appartenere a un’organizzazione.');
        }
        $orgRole = $organizationId === null ? null : ($role === 'admin' ? 'admin' : 'member');
        db()->prepare('INSERT INTO users (email, display_name, password_hash, role, organization_id, organization_role) VALUES (?, ?, ?, ?, ?, ?)')
            ->execute([$email, mb_substr($name, 0, 120), password_hash($password, PASSWORD_DEFAULT), $role, $organizationId, $orgRole]);
        api_json(['message' => 'Utente creato.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'user-update') {
        if (!$tenant['can_manage_accounts']) throw new RuntimeException('Operazione riservata agli amministratori.', 403);
        $id = (int) ($body['id'] ?? 0);
        $stmt = db()->prepare('SELECT id, email, display_name, role, organization_id, organization_role, is_active, approval_status FROM users WHERE id = ?');
        $stmt->execute([$id]);
        $target = $stmt->fetch();
        if (!$target) throw new RuntimeException('Utente non trovato.', 404);
        $isSuperAdmin = $user['role'] === 'super_admin';
        if (!$isSuperAdmin && ((int) ($target['organization_id'] ?? 0) !== (int) $tenant['organization_id'] || !in_array($target['role'], ['admin', 'user'], true))) {
            throw new RuntimeException('Utente non appartenente alla tua organizzazione.', 403);
        }
        $email = mb_strtolower(trim((string) ($body['email'] ?? '')));
        $name = trim((string) ($body['display_name'] ?? ''));
        $password = (string) ($body['password'] ?? '');
        $role = (string) ($body['role'] ?? 'user');
        $status = (string) ($body['approval_status'] ?? 'approved');
        $organizationId = isset($body['organization_id']) ? (int) $body['organization_id'] : null;
        if (!filter_var($email, FILTER_VALIDATE_EMAIL) || $name === '' || ($password !== '' && strlen($password) < 8)) {
            throw new RuntimeException('Dati utente non validi. La nuova password deve contenere almeno 8 caratteri.');
        }
        if (!in_array($status, ['pending', 'approved', 'rejected', 'banned'], true)) throw new RuntimeException('Stato utente non valido.');
        $duplicate = db()->prepare('SELECT id FROM users WHERE email = ? AND id <> ?');
        $duplicate->execute([$email, $id]);
        if ($duplicate->fetchColumn() !== false) throw new RuntimeException('Esiste già un account con questa email.');
        if ($id === (int) $user['id']) {
            $role = (string) $target['role'];
            $organizationId = $target['organization_id'] === null ? null : (int) $target['organization_id'];
            $status = (string) $target['approval_status'];
        } elseif ($isSuperAdmin) {
            if (!in_array($role, ['super_admin', 'admin', 'user'], true)) throw new RuntimeException('Ruolo non valido.');
            if ($role === 'super_admin') $organizationId = null;
        } else {
            if (!in_array($role, ['admin', 'user'], true)) throw new RuntimeException('Ruolo non valido.');
            $organizationId = (int) $tenant['organization_id'];
        }
        if ($role === 'admin' && $organizationId === null) throw new RuntimeException('Un amministratore deve appartenere a un’organizzazione.');
        if ($organizationId !== null) {
            $organization = db()->prepare('SELECT id FROM organizations WHERE id = ?');
            $organization->execute([$organizationId]);
            if ($organization->fetchColumn() === false) throw new RuntimeException('Organizzazione non trovata.');
        }
        $organizationRole = $organizationId === null ? null : ($role === 'admin' ? 'admin' : 'member');
        $params = [$email, mb_substr($name, 0, 120), $role, $organizationId, $organizationRole, $status, $status === 'approved' ? 1 : 0];
        $sql = 'UPDATE users SET email = ?, display_name = ?, role = ?, organization_id = ?, organization_role = ?, approval_status = ?, is_active = ?';
        if ($password !== '') {
            $sql .= ', password_hash = ?';
            $params[] = password_hash($password, PASSWORD_DEFAULT);
        }
        $sql .= ' WHERE id = ?';
        $params[] = $id;
        db()->prepare($sql)->execute($params);
        db()->prepare('DELETE FROM api_tokens WHERE user_id = ?')->execute([$id]);
        audit((int) $user['id'], 'user_updated_in_app', ['target_user_id' => $id]);
        if ($id === (int) $user['id']) {
            $user['email'] = $email;
            $user['display_name'] = mb_substr($name, 0, 120);
        }
        api_json(['message' => 'Utente aggiornato.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'user-delete-many') {
        if (!$tenant['can_manage_accounts']) throw new RuntimeException('Operazione riservata agli amministratori.', 403);
        $ids = api_clean_ids($body['ids'] ?? null, 'Selezione utenti non valida.');
        if (in_array((int) $user['id'], $ids, true)) throw new RuntimeException('Non puoi eliminare il tuo account.');
        $placeholders = implode(',', array_fill(0, count($ids), '?'));
        $params = $ids;
        $scope = '';
        if ($user['role'] !== 'super_admin') {
            $scope = " AND organization_id = ? AND role IN ('admin', 'user')";
            $params[] = (int) $tenant['organization_id'];
        }
        $pdo = db();
        $pdo->beginTransaction();
        try {
            $stmt = $pdo->prepare("SELECT id FROM users WHERE id IN ({$placeholders}){$scope} FOR UPDATE");
            $stmt->execute($params);
            $found = array_map('intval', $stmt->fetchAll(PDO::FETCH_COLUMN));
            sort($found); $expected = $ids; sort($expected);
            if ($found !== $expected) throw new RuntimeException('Uno o più utenti non sono gestibili da questo account.', 403);
            $personal = $pdo->prepare("SELECT user_id, front_storage_name, back_storage_name FROM cover_sets WHERE organization_id IS NULL AND user_id IN ({$placeholders})");
            $personal->execute($ids);
            $personalSets = $personal->fetchAll();
            $pdo->prepare("UPDATE cover_sets SET user_id = ? WHERE organization_id IS NOT NULL AND user_id IN ({$placeholders})")
                ->execute(array_merge([(int) $user['id']], $ids));
            $pdo->prepare("DELETE FROM users WHERE id IN ({$placeholders})")->execute($ids);
            audit((int) $user['id'], 'users_deleted_in_app', ['target_user_ids' => $ids]);
            $pdo->commit();
        } catch (Throwable $error) {
            if ($pdo->inTransaction()) $pdo->rollBack();
            throw $error;
        }
        foreach ($ids as $deletedId) {
            $sets = array_values(array_filter($personalSets, static fn(array $set): bool => (int) $set['user_id'] === $deletedId));
            api_remove_cover_files('users/' . $deletedId, $sets);
        }
        api_json(['message' => 'Utenti eliminati.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'user-toggle') {
        if (!$tenant['can_manage_accounts']) throw new RuntimeException('Operazione riservata agli amministratori.', 403);
        $id = (int) ($body['id'] ?? 0);
        if ($id === (int) $user['id']) throw new RuntimeException('Non puoi disattivare il tuo account.');
        if ($user['role'] === 'super_admin') {
            $stmt = db()->prepare('UPDATE users SET is_active = NOT is_active WHERE id = ?'); $stmt->execute([$id]);
        } else {
            $stmt = db()->prepare('UPDATE users SET is_active = NOT is_active WHERE id = ? AND organization_id = ?'); $stmt->execute([$id, $tenant['organization_id']]);
        }
        if ($stmt->rowCount() !== 1) throw new RuntimeException('Utente non trovato.');
        api_json(['message' => 'Stato utente aggiornato.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'user-approval') {
        if (!$tenant['can_manage_accounts']) throw new RuntimeException('Operazione riservata agli amministratori.', 403);
        $id = (int) ($body['id'] ?? 0);
        $status = (string) ($body['status'] ?? '');
        if ($id < 1 || !in_array($status, ['approved', 'rejected', 'banned'], true)) throw new RuntimeException('Approvazione non valida.');
        if ($id === (int) $user['id']) throw new RuntimeException('Non puoi modificare la tua approvazione.');
        if ($user['role'] === 'super_admin') {
            $stmt = db()->prepare('UPDATE users SET approval_status = ?, is_active = ? WHERE id = ?');
            $stmt->execute([$status, $status === 'approved' ? 1 : 0, $id]);
        } else {
            $stmt = db()->prepare("UPDATE users SET approval_status = ?, is_active = ? WHERE id = ? AND organization_id = ? AND role IN ('admin', 'user')");
            $stmt->execute([$status, $status === 'approved' ? 1 : 0, $id, $tenant['organization_id']]);
        }
        if ($stmt->rowCount() !== 1) throw new RuntimeException('Richiesta di registrazione non trovata.');
        db()->prepare('DELETE FROM api_tokens WHERE user_id = ?')->execute([$id]);
        audit((int) $user['id'], 'registration_' . $status, ['target_user_id' => $id]);
        $messages = ['approved' => 'Account approvato e attivo.', 'rejected' => 'Registrazione rifiutata.', 'banned' => 'Account bannato.'];
        api_json(['message' => $messages[$status], 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'organization-create') {
        if ($user['role'] !== 'super_admin') throw new RuntimeException('Operazione riservata ai Super Admin.', 403);
        $name = trim((string) ($body['name'] ?? '')); $slug = strtolower(trim((string) ($body['slug'] ?? '')));
        if ($name === '' || !preg_match('/^[a-z0-9][a-z0-9-]{1,78}[a-z0-9]$/', $slug)) throw new RuntimeException('Nome o slug non valido.');
        db()->prepare('INSERT INTO organizations (name, slug) VALUES (?, ?)')->execute([mb_substr($name, 0, 120), $slug]);
        api_json(['message' => 'Organizzazione creata.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'organization-update') {
        if ($user['role'] !== 'super_admin') throw new RuntimeException('Operazione riservata ai Super Admin.', 403);
        $id = (int) ($body['id'] ?? 0);
        $name = trim((string) ($body['name'] ?? ''));
        $slug = strtolower(trim((string) ($body['slug'] ?? '')));
        $active = filter_var($body['is_active'] ?? false, FILTER_VALIDATE_BOOL);
        if ($id < 1 || $name === '' || !preg_match('/^[a-z0-9][a-z0-9-]{1,78}[a-z0-9]$/', $slug)) throw new RuntimeException('Nome o slug non valido.');
        $duplicate = db()->prepare('SELECT id FROM organizations WHERE slug = ? AND id <> ?');
        $duplicate->execute([$slug, $id]);
        if ($duplicate->fetchColumn() !== false) throw new RuntimeException('Slug già utilizzato.');
        $stmt = db()->prepare('UPDATE organizations SET name = ?, slug = ?, is_active = ? WHERE id = ?');
        $stmt->execute([mb_substr($name, 0, 120), $slug, $active ? 1 : 0, $id]);
        if ($stmt->rowCount() < 1) {
            $exists = db()->prepare('SELECT id FROM organizations WHERE id = ?');
            $exists->execute([$id]);
            if ($exists->fetchColumn() === false) throw new RuntimeException('Organizzazione non trovata.', 404);
        }
        audit((int) $user['id'], 'organization_updated_in_app', ['organization_id' => $id]);
        api_json(['message' => 'Organizzazione aggiornata.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'organization-delete') {
        if ($user['role'] !== 'super_admin') throw new RuntimeException('Operazione riservata ai Super Admin.', 403);
        $id = (int) ($body['id'] ?? 0);
        $deleteUsers = filter_var($body['delete_users'] ?? false, FILTER_VALIDATE_BOOL);
        if ($id < 1) throw new RuntimeException('Organizzazione non valida.');
        $pdo = db();
        $pdo->beginTransaction();
        try {
            $lock = $pdo->prepare('SELECT id, name FROM organizations WHERE id = ? FOR UPDATE');
            $lock->execute([$id]);
            $organization = $lock->fetch();
            if (!$organization) throw new RuntimeException('Organizzazione non trovata.', 404);
            $covers = $pdo->prepare('SELECT front_storage_name, back_storage_name FROM cover_sets WHERE organization_id = ?');
            $covers->execute([$id]);
            $coverSets = $covers->fetchAll();
            if ($deleteUsers) {
                $pdo->prepare('DELETE FROM users WHERE organization_id = ?')->execute([$id]);
            } else {
                $pdo->prepare("UPDATE users SET organization_id = NULL, organization_role = NULL, role = 'user' WHERE organization_id = ?")->execute([$id]);
            }
            audit((int) $user['id'], 'organization_deleted_in_app', ['organization_id' => $id, 'delete_users' => $deleteUsers], $id);
            $pdo->prepare('DELETE FROM organizations WHERE id = ?')->execute([$id]);
            $pdo->commit();
        } catch (Throwable $error) {
            if ($pdo->inTransaction()) $pdo->rollBack();
            throw $error;
        }
        api_remove_cover_files('organizations/' . $id, $coverSets);
        api_json(['message' => $deleteUsers ? 'Organizzazione e utenti eliminati.' : 'Organizzazione eliminata; gli utenti sono ora account personali.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'organization-toggle') {
        if ($user['role'] !== 'super_admin') throw new RuntimeException('Operazione riservata ai Super Admin.', 403);
        db()->prepare('UPDATE organizations SET is_active = NOT is_active WHERE id = ?')->execute([(int) ($body['id'] ?? 0)]);
        api_json(['message' => 'Stato organizzazione aggiornato.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'ai-save') {
        $provider = (string) ($body['provider'] ?? '');
        $authMode = (string) ($body['auth_mode'] ?? 'api_key');
        if (!in_array($provider, ['openai', 'gemini', 'openrouter', 'compatible'], true) || !in_array($authMode, ['api_key', 'chatgpt_oauth'], true)) throw new RuntimeException('Provider AI non valido.');
        if ($provider !== 'openai') $authMode = 'api_key';
        $base = trim((string) ($body['base_url'] ?? '')); $model = trim((string) ($body['model'] ?? '')); $secret = (string) ($body['secret'] ?? '');
        if (strlen($base) > 512 || strlen($model) > 160) throw new RuntimeException('Configurazione AI troppo lunga.');
        $cipher = $secret === '' ? null : api_encrypt($secret);
        $sharedIds = array_values(array_unique(array_map('intval', is_array($body['shared_user_ids'] ?? null) ? $body['shared_user_ids'] : [])));
        if (count($sharedIds) > 10_000) throw new RuntimeException('Troppi utenti selezionati.');
        if ($sharedIds !== [] && ($user['role'] !== 'admin' || $user['organization_id'] === null)) {
            throw new RuntimeException('Solo un Admin di organizzazione puÃ² condividere il provider AI.', 403);
        }
        $pdo = db();
        $pdo->beginTransaction();
        try {
            $pdo->prepare('INSERT INTO ai_provider_settings (user_id, provider, auth_mode, base_url, model, secret_cipher) VALUES (?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE provider=VALUES(provider), auth_mode=VALUES(auth_mode), base_url=VALUES(base_url), model=VALUES(model), secret_cipher=COALESCE(VALUES(secret_cipher), secret_cipher)')
                ->execute([(int) $user['id'], $provider, $authMode, $base, $model, $cipher]);
            if ($user['role'] === 'admin' && $user['organization_id'] !== null) {
                if ($sharedIds !== []) {
                    $placeholders = implode(',', array_fill(0, count($sharedIds), '?'));
                    $params = array_merge($sharedIds, [(int) $user['organization_id'], (int) $user['id']]);
                    $stmt = $pdo->prepare("SELECT id FROM users WHERE id IN ({$placeholders}) AND organization_id = ? AND id <> ? AND is_active = 1 AND approval_status = 'approved'");
                    $stmt->execute($params);
                    $validIds = array_map('intval', $stmt->fetchAll(PDO::FETCH_COLUMN));
                    sort($validIds);
                    $expected = $sharedIds;
                    sort($expected);
                    if ($validIds !== $expected) throw new RuntimeException('Uno o piÃ¹ utenti non appartengono alla tua organizzazione o non sono attivi.');
                }
                $pdo->prepare('DELETE FROM ai_provider_shares WHERE owner_user_id = ?')->execute([(int) $user['id']]);
                $share = $pdo->prepare('INSERT INTO ai_provider_shares (owner_user_id, target_user_id) VALUES (?, ?) ON DUPLICATE KEY UPDATE owner_user_id = VALUES(owner_user_id)');
                foreach ($sharedIds as $targetId) $share->execute([(int) $user['id'], $targetId]);
            }
            $pdo->commit();
        } catch (Throwable $error) {
            if ($pdo->inTransaction()) $pdo->rollBack();
            throw $error;
        }
        api_json(['message' => 'Provider AI salvato.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'ai-source') {
        $source = (string) ($body['source'] ?? 'personal');
        if (!in_array($source, ['personal', 'organization'], true)) throw new RuntimeException('Sorgente AI non valida.');
        if ($source === 'organization') api_ai_config($user, 'organization');
        db()->prepare('INSERT INTO user_preferences (user_id, ai_source) VALUES (?, ?) ON DUPLICATE KEY UPDATE ai_source=VALUES(ai_source)')
            ->execute([(int) $user['id'], $source]);
        api_json(['message' => 'Sorgente AI aggiornata.', 'snapshot' => api_snapshot($user)]);
    }
    if ($action === 'ai-models') {
        $source = (string) ($body['source'] ?? 'personal');
        $draftProvider = (string) ($body['provider'] ?? '');
        $draftAuth = (string) ($body['auth_mode'] ?? '');
        $draftBase = trim((string) ($body['base_url'] ?? ''));
        $draftSecret = (string) ($body['secret'] ?? '');
        try {
            $config = api_ai_config($user, $source);
        } catch (RuntimeException $error) {
            if ($source === 'organization') throw $error;
            if ($draftSecret === '' && !($draftProvider === 'compatible' && $draftBase !== '')) throw $error;
            $config = ['provider' => 'openai', 'auth_mode' => 'api_key', 'base_url' => '', 'model' => '', 'secret' => $draftSecret];
        }
        if ($source !== 'organization') {
            if (in_array($draftProvider, ['openai', 'gemini', 'openrouter', 'compatible'], true)) $config['provider'] = $draftProvider;
            if (in_array($draftAuth, ['api_key', 'chatgpt_oauth'], true)) $config['auth_mode'] = $draftAuth;
            if ($draftBase !== '') $config['base_url'] = $draftBase;
            if ($draftSecret !== '') $config['secret'] = $draftSecret;
        }
        api_json(['models' => api_ai_models($config)]);
    }
    if ($action === 'ai-run') api_json(['result' => api_ai_run(api_ai_config($user), $body)]);
    if ($action === 'chatgpt-auth-start') {
        $clientId = (string) env_value('OPENAI_CLIENT_ID', '');
        if ($clientId === '') throw new RuntimeException('OPENAI_CLIENT_ID non configurato sul server.');
        $state = rtrim(strtr(base64_encode(random_bytes(32)), '+/', '-_'), '=');
        $verifier = rtrim(strtr(base64_encode(random_bytes(64)), '+/', '-_'), '=');
        $challenge = rtrim(strtr(base64_encode(hash('sha256', $verifier, true)), '+/', '-_'), '=');
        db()->prepare('DELETE FROM oauth_transactions WHERE expires_at <= CURRENT_TIMESTAMP')->execute();
        db()->prepare('INSERT INTO oauth_transactions (state_hash, user_id, verifier_cipher, expires_at) VALUES (?, ?, ?, DATE_ADD(CURRENT_TIMESTAMP, INTERVAL 10 MINUTE))')
            ->execute([hash('sha256', $state), (int) $user['id'], api_encrypt($verifier)]);
        $redirect = rtrim((string) env_value('APP_URL', ''), '/') . '/app-api.php?action=chatgpt-callback';
        $query = http_build_query(['client_id' => $clientId, 'response_type' => 'code', 'redirect_uri' => $redirect, 'scope' => 'openid profile email offline_access chatgpt.tokens.use.direct', 'state' => $state, 'code_challenge' => $challenge, 'code_challenge_method' => 'S256', 'resource' => 'https://api.openai.com/v1']);
        api_json(['url' => 'https://auth.openai.com/api/accounts/authorize?' . $query]);
    }
    throw new RuntimeException('Azione non riconosciuta.', 404);
} catch (Throwable $error) {
    $code = $error->getCode();
    $status = is_int($code) && $code >= 400 && $code <= 599 ? $code : (http_response_code() >= 400 ? http_response_code() : 422);
    api_json(['error' => $error->getMessage()], $status);
}
