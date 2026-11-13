<?php
declare(strict_types=1);

const DOCUMENT_SHARE_RECIPIENT_LIMIT = 100;

function document_file_path(int $organizationId, string $storageName): string
{
    if ($organizationId < 1 || preg_match('/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-a[0-9a-f]{3}-[0-9a-f]{12}$/', $storageName) !== 1) {
        throw new RuntimeException('Identificativo documento non valido.');
    }
    return storage_path("organizations/{$organizationId}/documents/{$storageName}.pdf");
}

function remove_organization_document_files(int $organizationId, array $storageNames): void
{
    foreach ($storageNames as $storageName) {
        if (!is_string($storageName)) {
            error_log('easyPDF: identificativo documento non valido durante la pulizia.');
            continue;
        }
        try {
            $path = document_file_path($organizationId, $storageName);
        } catch (RuntimeException $error) {
            error_log('easyPDF: identificativo documento non valido durante la pulizia: ' . $error->getMessage());
            continue;
        }
        if (is_file($path) && !@unlink($path)) {
            error_log('easyPDF: impossibile rimuovere un documento condiviso dell’organizzazione ' . $organizationId);
        }
    }
}

function document_original_name(string $name): string
{
    $clean = trim(basename(str_replace('\\', '/', $name)));
    if ($clean === '') {
        $clean = 'documento.pdf';
    }
    if (!str_ends_with(mb_strtolower($clean), '.pdf')) {
        $clean .= '.pdf';
    }
    return mb_substr($clean, 0, 255);
}

function store_shared_document_bytes(int $organizationId, string $storageName, string $bytes): string
{
    if (strlen($bytes) < 5 || !str_starts_with($bytes, '%PDF-')) {
        throw new RuntimeException('Il documento da condividere non è un PDF valido.');
    }
    $target = document_file_path($organizationId, $storageName);
    ensure_directory(dirname($target));
    $temporary = $target . '.tmp-' . bin2hex(random_bytes(8));
    $written = file_put_contents($temporary, $bytes, LOCK_EX);
    if (!is_int($written) || $written !== strlen($bytes)) {
        @unlink($temporary);
        throw new RuntimeException('Impossibile salvare il documento condiviso.');
    }
    @chmod($temporary, 0640);
    if (!rename($temporary, $target)) {
        @unlink($temporary);
        throw new RuntimeException('Impossibile finalizzare il documento condiviso.');
    }
    return $target;
}

function document_share_members(array $user): array
{
    if ($user['organization_id'] === null) {
        return [];
    }
    $stmt = db()->prepare(
        "SELECT id, display_name FROM users WHERE organization_id = ? AND id <> ? "
        . "AND is_active = 1 AND approval_status = 'approved' ORDER BY display_name, id"
    );
    $stmt->execute([(int) $user['organization_id'], (int) $user['id']]);
    return array_map(static fn(array $member): array => [
        'id' => (int) $member['id'],
        'display_name' => (string) $member['display_name'],
    ], $stmt->fetchAll());
}

function document_notifications(array $user): array
{
    if ($user['organization_id'] === null) {
        return [];
    }
    $stmt = db()->prepare(
        'SELECT id, document_id, actor_name_snapshot, document_name_snapshot, read_at, created_at '
        . 'FROM notifications WHERE organization_id = ? AND recipient_user_id = ? '
        . 'ORDER BY created_at DESC, id DESC LIMIT 500'
    );
    $stmt->execute([(int) $user['organization_id'], (int) $user['id']]);
    return array_map(static fn(array $item): array => [
        'id' => (int) $item['id'],
        'document_id' => $item['document_id'] === null ? null : (int) $item['document_id'],
        'actor_name' => (string) $item['actor_name_snapshot'],
        'document_name' => (string) $item['document_name_snapshot'],
        'is_read' => $item['read_at'] !== null,
        'created_at' => (string) $item['created_at'],
    ], $stmt->fetchAll());
}

function received_shared_documents(array $user): array
{
    if ($user['organization_id'] === null) {
        return [];
    }
    $stmt = db()->prepare(
        'SELECT d.id, d.original_name, d.byte_size, d.created_at, s.opened_at, '
        . 'COALESCE(sender.display_name, owner.display_name, ?) AS sender_name '
        . 'FROM document_shares s JOIN shared_documents d ON d.id = s.document_id '
        . 'LEFT JOIN users sender ON sender.id = s.shared_by_user_id '
        . 'LEFT JOIN users owner ON owner.id = d.owner_user_id '
        . 'WHERE d.organization_id = ? AND s.recipient_user_id = ? '
        . 'ORDER BY s.created_at DESC, d.id DESC LIMIT 500'
    );
    $stmt->execute(['Utente eliminato', (int) $user['organization_id'], (int) $user['id']]);
    return array_map(static fn(array $item): array => [
        'id' => (int) $item['id'],
        'name' => (string) $item['original_name'],
        'byte_size' => (int) $item['byte_size'],
        'sender_name' => (string) $item['sender_name'],
        'is_opened' => $item['opened_at'] !== null,
        'created_at' => (string) $item['created_at'],
    ], $stmt->fetchAll());
}

function create_shared_document(array $user, string $name, string $bytes, array $recipientIds): void
{
    $organizationId = $user['organization_id'] === null ? 0 : (int) $user['organization_id'];
    if ($organizationId < 1) {
        throw new RuntimeException('La condivisione richiede un account appartenente a un’organizzazione.');
    }
    if (strlen($bytes) < 5 || !str_starts_with($bytes, '%PDF-')) {
        throw new RuntimeException('Il documento da condividere non è un PDF valido.');
    }
    if ($recipientIds === [] || count($recipientIds) > DOCUMENT_SHARE_RECIPIENT_LIMIT) {
        throw new RuntimeException('Seleziona almeno un destinatario valido.');
    }
    $ids = [];
    foreach ($recipientIds as $id) {
        if ((!is_int($id) && !ctype_digit((string) $id)) || (int) $id < 1 || (int) $id === (int) $user['id']) {
            throw new RuntimeException('Selezione destinatari non valida.');
        }
        $clean = (int) $id;
        if (!in_array($clean, $ids, true)) {
            $ids[] = $clean;
        }
    }
    if ($ids === []) {
        throw new RuntimeException('Seleziona almeno un destinatario valido.');
    }
    $placeholders = implode(',', array_fill(0, count($ids), '?'));
    $memberStmt = db()->prepare(
        "SELECT id FROM users WHERE id IN ({$placeholders}) AND organization_id = ? "
        . "AND is_active = 1 AND approval_status = 'approved'"
    );
    $memberStmt->execute(array_merge($ids, [$organizationId]));
    $found = array_map('intval', $memberStmt->fetchAll(PDO::FETCH_COLUMN));
    sort($found);
    $expected = $ids;
    sort($expected);
    if ($found !== $expected) {
        throw new RuntimeException('Uno o più destinatari non appartengono alla tua organizzazione.');
    }

    $storageName = random_id();
    $target = store_shared_document_bytes($organizationId, $storageName, $bytes);

    $pdo = db();
    try {
        $pdo->beginTransaction();
        $originalName = document_original_name($name);
        $stmt = $pdo->prepare(
            'INSERT INTO shared_documents (organization_id, owner_user_id, original_name, storage_name, sha256, byte_size) '
            . 'VALUES (?, ?, ?, ?, ?, ?)'
        );
        $stmt->execute([$organizationId, (int) $user['id'], $originalName, $storageName, hash('sha256', $bytes), strlen($bytes)]);
        $documentId = (int) $pdo->lastInsertId();
        $share = $pdo->prepare('INSERT INTO document_shares (document_id, recipient_user_id, shared_by_user_id) VALUES (?, ?, ?)');
        $notify = $pdo->prepare(
            "INSERT INTO notifications (organization_id, recipient_user_id, actor_user_id, document_id, notification_type, actor_name_snapshot, document_name_snapshot) "
            . "VALUES (?, ?, ?, ?, 'document_shared', ?, ?)"
        );
        foreach ($ids as $recipientId) {
            $share->execute([$documentId, $recipientId, (int) $user['id']]);
            $notify->execute([$organizationId, $recipientId, (int) $user['id'], $documentId, (string) $user['display_name'], $originalName]);
        }
        $pdo->commit();
    } catch (Throwable $error) {
        if ($pdo->inTransaction()) {
            $pdo->rollBack();
        }
        @unlink($target);
        throw $error;
    }
}

function shared_document_for_download(array $user, int $documentId): array
{
    if ($documentId < 1 || $user['organization_id'] === null) {
        throw new RuntimeException('Documento condiviso non trovato.', 404);
    }
    $stmt = db()->prepare(
        'SELECT d.id, d.organization_id, d.original_name, d.storage_name, d.byte_size '
        . 'FROM shared_documents d JOIN document_shares s ON s.document_id = d.id '
        . 'WHERE d.id = ? AND d.organization_id = ? AND s.recipient_user_id = ? LIMIT 1'
    );
    $stmt->execute([$documentId, (int) $user['organization_id'], (int) $user['id']]);
    $document = $stmt->fetch();
    if (!$document) {
        throw new RuntimeException('Documento condiviso non trovato.', 404);
    }
    $path = document_file_path((int) $document['organization_id'], (string) $document['storage_name']);
    if (!is_file($path) || !is_readable($path)) {
        throw new RuntimeException('File condiviso non disponibile.', 404);
    }
    db()->prepare('UPDATE document_shares SET opened_at = COALESCE(opened_at, CURRENT_TIMESTAMP) WHERE document_id = ? AND recipient_user_id = ?')
        ->execute([$documentId, (int) $user['id']]);
    return ['path' => $path, 'name' => (string) $document['original_name'], 'size' => (int) $document['byte_size']];
}

function mark_notification_read(array $user, int $notificationId): void
{
    if ($notificationId < 1 || $user['organization_id'] === null) {
        throw new RuntimeException('Notifica non valida.');
    }
    $stmt = db()->prepare(
        'UPDATE notifications SET read_at = COALESCE(read_at, CURRENT_TIMESTAMP) '
        . 'WHERE id = ? AND organization_id = ? AND recipient_user_id = ?'
    );
    $stmt->execute([$notificationId, (int) $user['organization_id'], (int) $user['id']]);
    if ($stmt->rowCount() !== 1) {
        $exists = db()->prepare('SELECT id FROM notifications WHERE id = ? AND organization_id = ? AND recipient_user_id = ?');
        $exists->execute([$notificationId, (int) $user['organization_id'], (int) $user['id']]);
        if ($exists->fetchColumn() === false) {
            throw new RuntimeException('Notifica non trovata.', 404);
        }
    }
}

function mark_all_notifications_read(array $user): void
{
    if ($user['organization_id'] === null) {
        return;
    }
    db()->prepare(
        'UPDATE notifications SET read_at = CURRENT_TIMESTAMP '
        . 'WHERE organization_id = ? AND recipient_user_id = ? AND read_at IS NULL'
    )->execute([(int) $user['organization_id'], (int) $user['id']]);
}
