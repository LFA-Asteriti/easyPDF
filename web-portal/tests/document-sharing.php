<?php
declare(strict_types=1);

$storageRoot = sys_get_temp_dir() . DIRECTORY_SEPARATOR . 'easypdf-document-sharing-' . bin2hex(random_bytes(8));
putenv('EASYPDF_STORAGE=' . $storageRoot);

require __DIR__ . '/../src/bootstrap.php';
require __DIR__ . '/../src/documents.php';

function sharing_assert(bool $condition, string $message): void
{
    if (!$condition) {
        throw new RuntimeException($message);
    }
}

$organizationId = null;
$otherOrganizationId = null;
$createdUserIds = [];
try {
    $suffix = bin2hex(random_bytes(6));
    db()->prepare('INSERT INTO organizations (name, slug) VALUES (?, ?)')->execute(['Test condivisione', 'test-share-' . $suffix]);
    $organizationId = (int) db()->lastInsertId();
    $insertUser = db()->prepare(
        "INSERT INTO users (email, display_name, password_hash, role, organization_id, organization_role, is_active, approval_status) "
        . "VALUES (?, ?, ?, 'user', ?, 'member', 1, 'approved')"
    );
    $insertUser->execute(["sender-{$suffix}@example.test", 'Mittente test', password_hash('test-password', PASSWORD_DEFAULT), $organizationId]);
    $senderId = (int) db()->lastInsertId();
    $createdUserIds[] = $senderId;
    $insertUser->execute(["recipient-{$suffix}@example.test", 'Destinatario test', password_hash('test-password', PASSWORD_DEFAULT), $organizationId]);
    $recipientId = (int) db()->lastInsertId();
    $createdUserIds[] = $recipientId;

    db()->prepare('INSERT INTO organizations (name, slug) VALUES (?, ?)')->execute(['Altra organizzazione', 'test-other-' . $suffix]);
    $otherOrganizationId = (int) db()->lastInsertId();
    $insertUser->execute(["outsider-{$suffix}@example.test", 'Esterno test', password_hash('test-password', PASSWORD_DEFAULT), $otherOrganizationId]);
    $outsiderId = (int) db()->lastInsertId();
    $createdUserIds[] = $outsiderId;

    $sender = [
        'id' => $senderId,
        'display_name' => 'Mittente test',
        'organization_id' => $organizationId,
    ];
    $recipient = [
        'id' => $recipientId,
        'display_name' => 'Destinatario test',
        'organization_id' => $organizationId,
    ];
    $outsider = [
        'id' => $outsiderId,
        'display_name' => 'Esterno test',
        'organization_id' => $otherOrganizationId,
    ];
    $pdf = "%PDF-1.7\n% shared integration fixture\n%%EOF\n";

    create_shared_document($sender, 'bozza modificata.pdf', $pdf, [$recipientId]);
    $documents = received_shared_documents($recipient);
    sharing_assert(count($documents) === 1, 'Il destinatario non vede esattamente un documento condiviso.');
    sharing_assert($documents[0]['name'] === 'bozza modificata.pdf', 'Il nome del documento non è stato conservato.');
    $notifications = document_notifications($recipient);
    sharing_assert(count($notifications) === 1 && !$notifications[0]['is_read'], 'La notifica non letta non è stata creata.');
    sharing_assert($notifications[0]['actor_name'] === 'Mittente test', 'Il mittente della notifica non corrisponde.');

    $download = shared_document_for_download($recipient, (int) $documents[0]['id']);
    sharing_assert(file_get_contents($download['path']) === $pdf, 'Il file protetto restituito al destinatario è diverso dalla copia salvata.');
    sharing_assert(str_starts_with($download['path'], $storageRoot . DIRECTORY_SEPARATOR), 'Il file non risiede nello storage locale simulato.');

    $forbidden = false;
    try {
        shared_document_for_download($outsider, (int) $documents[0]['id']);
    } catch (RuntimeException $error) {
        $forbidden = $error->getCode() === 404;
    }
    sharing_assert($forbidden, 'Un utente di un’altra organizzazione non deve poter aprire il documento.');

    mark_notification_read($recipient, (int) $notifications[0]['id']);
    $notifications = document_notifications($recipient);
    sharing_assert(count($notifications) === 1 && $notifications[0]['is_read'], 'La notifica non è stata segnata come letta.');

    fwrite(STDOUT, "Test integrazione condivisione completato nello storage simulato {$storageRoot}\n");
} finally {
    $deleteUser = db()->prepare('DELETE FROM users WHERE id = ?');
    foreach ($createdUserIds as $id) {
        $deleteUser->execute([$id]);
    }
    if (is_int($organizationId) || is_int($otherOrganizationId)) {
        $delete = db()->prepare('DELETE FROM organizations WHERE id = ?');
        foreach ([$organizationId, $otherOrganizationId] as $id) {
            if (is_int($id)) {
                $delete->execute([$id]);
            }
        }
    }
    remove_tree($storageRoot);
}
