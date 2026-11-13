<?php
declare(strict_types=1);

require __DIR__ . '/../src/bootstrap.php';

header('Content-Type: application/json; charset=utf-8');
header('Cache-Control: private, no-store');

/** @return list<string> */
function clean_tool_ids(mixed $value, string $message): array
{
    if (!is_array($value) || count($value) > 256) {
        throw new RuntimeException($message);
    }
    $clean = [];
    foreach ($value as $id) {
        if (!is_string($id) || !preg_match('/^[a-z0-9._-]{1,64}$/', $id) || in_array($id, $clean, true)) {
            throw new RuntimeException($message);
        }
        $clean[] = $id;
    }
    return $clean;
}

try {
    $user = current_user();
    if ($user === null) {
        http_response_code(401);
        throw new RuntimeException('Autenticazione richiesta.');
    }
    $userId = (int) $user['id'];

    if ($_SERVER['REQUEST_METHOD'] === 'GET') {
        $stmt = db()->prepare('SELECT tool_order_json, recommended_tools_json, menu_tool_order_json FROM user_preferences WHERE user_id = ?');
        $stmt->execute([$userId]);
        $saved = $stmt->fetch();
        echo json_encode([
            'csrf' => csrf_token(),
            'tool_order' => is_array($saved) && is_string($saved['tool_order_json'] ?? null)
                ? json_decode($saved['tool_order_json'], true, 32, JSON_THROW_ON_ERROR)
                : null,
            'recommended_tools' => is_array($saved) && is_string($saved['recommended_tools_json'] ?? null)
                ? json_decode($saved['recommended_tools_json'], true, 32, JSON_THROW_ON_ERROR)
                : null,
            'menu_tool_order' => is_array($saved) && is_string($saved['menu_tool_order_json'] ?? null)
                ? json_decode($saved['menu_tool_order_json'], true, 64, JSON_THROW_ON_ERROR)
                : null,
        ], JSON_THROW_ON_ERROR);
        exit;
    }

    if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
        http_response_code(405);
        header('Allow: GET, POST');
        throw new RuntimeException('Metodo non consentito.');
    }

    $body = json_decode((string) file_get_contents('php://input'), true, 32, JSON_THROW_ON_ERROR);
    if (!is_array($body)) {
        throw new RuntimeException('Preferenze non valide.');
    }
    $_POST['csrf'] = is_string($body['csrf'] ?? null) ? $body['csrf'] : '';
    verify_csrf();
    $hasOrder = array_key_exists('tool_order', $body);
    $hasRecommended = array_key_exists('recommended_tools', $body);
    $hasMenuOrder = array_key_exists('menu_tool_order', $body);
    if (!$hasOrder && !$hasRecommended && !$hasMenuOrder) {
        throw new RuntimeException('Nessuna preferenza dei tool specificata.');
    }
    $order = $hasOrder ? clean_tool_ids($body['tool_order'], 'Ordine dei tool non valido.') : null;
    $recommended = $hasRecommended ? clean_tool_ids($body['recommended_tools'], 'Strumenti consigliati non validi.') : null;
    $menuOrder = $hasMenuOrder ? clean_tool_ids($body['menu_tool_order'], 'Ordine dei menu Modifica/Converti non valido.') : null;
    $encodedOrder = $order === null ? null : json_encode($order, JSON_THROW_ON_ERROR);
    $encodedRecommended = $recommended === null ? null : json_encode($recommended, JSON_THROW_ON_ERROR);
    $encodedMenuOrder = $menuOrder === null ? null : json_encode($menuOrder, JSON_THROW_ON_ERROR);
    db()->prepare(
        'INSERT INTO user_preferences (user_id, tool_order_json, recommended_tools_json, menu_tool_order_json) VALUES (?, ?, ?, ?) '
        . 'ON DUPLICATE KEY UPDATE '
        . 'tool_order_json = COALESCE(VALUES(tool_order_json), tool_order_json), '
        . 'recommended_tools_json = COALESCE(VALUES(recommended_tools_json), recommended_tools_json), '
        . 'menu_tool_order_json = COALESCE(VALUES(menu_tool_order_json), menu_tool_order_json)'
    )->execute([$userId, $encodedOrder, $encodedRecommended, $encodedMenuOrder]);
    if ($order !== null) {
        audit($userId, 'tool_order_changed', ['count' => count($order)]);
    }
    if ($recommended !== null) {
        audit($userId, 'recommended_tools_changed', ['count' => count($recommended)]);
    }
    if ($menuOrder !== null) {
        audit($userId, 'menu_tool_order_changed', ['count' => count($menuOrder)]);
    }
    echo json_encode(['ok' => true], JSON_THROW_ON_ERROR);
} catch (Throwable $error) {
    http_response_code($error instanceof JsonException ? 400 : (http_response_code() >= 400 ? http_response_code() : 422));
    echo json_encode(['error' => $error->getMessage()], JSON_THROW_ON_ERROR);
}
