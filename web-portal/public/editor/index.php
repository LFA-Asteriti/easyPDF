<?php
declare(strict_types=1);

require dirname(__DIR__, 2) . '/src/bootstrap.php';
require_user();

$requestPath = (string) (parse_url($_SERVER['REQUEST_URI'] ?? '/editor/', PHP_URL_PATH) ?: '/editor/');
$relative = rawurldecode((string) preg_replace('#^/editor/?#', '', $requestPath));
if ($relative === '' || str_ends_with($relative, '/')) {
    $relative .= 'index.html';
}
if (str_contains($relative, "\0") || str_contains($relative, '\\') || preg_match('#(^|/)\.\.(/|$)#', $relative)) {
    http_response_code(400);
    exit('Percorso non valido.');
}

$extension = strtolower(pathinfo($relative, PATHINFO_EXTENSION));
$mimeTypes = [
    'html' => 'text/html; charset=utf-8',
    'js' => 'text/javascript; charset=utf-8',
    'mjs' => 'text/javascript; charset=utf-8',
    'wasm' => 'application/wasm',
    'css' => 'text/css; charset=utf-8',
    'json' => 'application/json; charset=utf-8',
    'svg' => 'image/svg+xml',
    'png' => 'image/png',
    'ico' => 'image/x-icon',
    'webmanifest' => 'application/manifest+json',
    'woff2' => 'font/woff2',
];
if (!isset($mimeTypes[$extension])) {
    http_response_code(404);
    exit('Risorsa non trovata.');
}

$root = realpath(dirname(__DIR__, 2) . '/editor-dist');
$file = $root === false ? false : realpath($root . DIRECTORY_SEPARATOR . str_replace('/', DIRECTORY_SEPARATOR, $relative));
if ($root === false || $file === false || !is_file($file) || !str_starts_with($file, $root . DIRECTORY_SEPARATOR)) {
    http_response_code(404);
    exit('Editor non compilato o risorsa non trovata.');
}

$contents = null;
$scriptPolicy = "'self' 'wasm-unsafe-eval'";
if ($extension === 'html') {
    $contents = file_get_contents($file);
    if ($contents === false) {
        http_response_code(500);
        exit('Impossibile leggere l’editor compilato.');
    }

    // Trunk generates the WASM bootstrap as an inline module. Authorize exactly
    // that generated code instead of weakening the policy with unsafe-inline.
    if (preg_match_all('#<script\b[^>]*>(.*?)</script>#is', $contents, $scripts) !== false) {
        foreach ($scripts[1] as $script) {
            $scriptPolicy .= " 'sha256-" . base64_encode(hash('sha256', $script, true)) . "'";
        }
    }
}

header("Content-Security-Policy: default-src 'self'; script-src {$scriptPolicy}; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self' blob:; worker-src 'self' blob:; font-src 'self'; object-src 'none'; frame-ancestors 'none'; base-uri 'self'");
header('Content-Type: ' . $mimeTypes[$extension]);
header('Content-Length: ' . ($contents === null ? filesize($file) : strlen($contents)));
header($extension === 'html' ? 'Cache-Control: private, no-store' : 'Cache-Control: private, max-age=31536000, immutable');
if ($contents === null) {
    readfile($file);
} else {
    echo $contents;
}
