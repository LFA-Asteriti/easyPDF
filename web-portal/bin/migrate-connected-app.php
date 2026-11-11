<?php
declare(strict_types=1);

require __DIR__ . '/../src/bootstrap.php';

try {
    foreach (['migrate-connected-app.sql', 'migrate-document-sharing.sql'] as $migration) {
        $sql = file_get_contents(__DIR__ . '/../database/' . $migration);
        if (!is_string($sql) || trim($sql) === '') {
            throw new RuntimeException("Migrazione {$migration} non leggibile.");
        }
        $statements = preg_split('/;\s*(?:\r?\n|$)/', $sql) ?: [];
        foreach ($statements as $statement) {
            if (trim($statement) !== '') {
                db()->exec($statement);
            }
        }
    }
    fwrite(STDOUT, "Migrazione connected-app applicata.\n");
} catch (Throwable $error) {
    fwrite(STDERR, "Migrazione connected-app fallita: {$error->getMessage()}\n");
    exit(1);
}
