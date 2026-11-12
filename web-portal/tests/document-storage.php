<?php
declare(strict_types=1);

$root = sys_get_temp_dir() . DIRECTORY_SEPARATOR . 'easypdf-document-storage-' . bin2hex(random_bytes(8));
putenv('APP_ENV=testing');
putenv('APP_SECRET=document-storage-test-secret-that-is-not-used');
putenv('EASYPDF_STORAGE=' . $root);

require __DIR__ . '/../src/bootstrap.php';
require __DIR__ . '/../src/documents.php';

function test_assert(bool $condition, string $message): void
{
    if (!$condition) {
        throw new RuntimeException($message);
    }
}

try {
    $organizationId = 42;
    $storageName = '01234567-89ab-4cde-af01-23456789abcd';
    $bytes = "%PDF-1.7\n% isolated storage simulation\n%%EOF\n";
    $path = store_shared_document_bytes($organizationId, $storageName, $bytes);
    $expectedDirectory = $root . DIRECTORY_SEPARATOR . 'organizations' . DIRECTORY_SEPARATOR . '42' . DIRECTORY_SEPARATOR . 'documents';

    test_assert(dirname($path) === $expectedDirectory, 'Il documento non è stato confinato nella cartella simulata dell’organizzazione.');
    test_assert(is_file($path), 'Il documento simulato non è stato creato.');
    test_assert(file_get_contents($path) === $bytes, 'I byte del documento simulato non corrispondono.');
    test_assert(!is_file(EASYPDF_ROOT . '/public/' . basename($path)), 'Il documento non deve essere scritto nella root pubblica.');

    $rejected = false;
    try {
        document_file_path($organizationId, '../evasione');
    } catch (RuntimeException) {
        $rejected = true;
    }
    test_assert($rejected, 'Un nome di storage non sicuro deve essere rifiutato.');

    $otherStorageName = '01234567-89ab-4cde-af01-23456789abce';
    $otherPath = store_shared_document_bytes($organizationId + 1, $otherStorageName, $bytes);
    remove_organization_document_files($organizationId, [$storageName]);
    test_assert(!is_file($path), 'La rimozione dell’organizzazione deve cancellare il PDF condiviso.');
    test_assert(is_file($otherPath), 'I documenti di altre organizzazioni devono restare intatti.');

    fwrite(STDOUT, "Test storage documenti condivisi completato: {$path}\n");
} finally {
    remove_tree($root);
}
