<?php
declare(strict_types=1);

// Local integration-test provider. Never routed by the production document root.
header('Content-Type: application/json; charset=utf-8');
$path = (string) parse_url((string) ($_SERVER['REQUEST_URI'] ?? ''), PHP_URL_PATH);
if ($_SERVER['REQUEST_METHOD'] === 'GET' && $path === '/v1/models') {
    echo json_encode(['data' => [['id' => 'easyPDF-test-model']]], JSON_THROW_ON_ERROR);
    exit;
}
if ($_SERVER['REQUEST_METHOD'] === 'POST' && $path === '/v1/chat/completions') {
    $body = json_decode((string) file_get_contents('php://input'), true, 32, JSON_THROW_ON_ERROR);
    $prompt = (string) ($body['messages'][0]['content'] ?? '');
    $kind = str_starts_with($prompt, 'Translate') ? 'translate' : (str_starts_with($prompt, 'Answer') ? 'ask' : 'summary');
    echo json_encode(['choices' => [['message' => ['content' => "mock-{$kind}-ok"]]]], JSON_THROW_ON_ERROR);
    exit;
}
http_response_code(404);
echo json_encode(['error' => 'not found'], JSON_THROW_ON_ERROR);
