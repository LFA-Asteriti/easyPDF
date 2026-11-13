<?php
declare(strict_types=1);

function user_cover_sets(array $tenant): array
{
    if ($tenant['organization_id'] === null) {
        $stmt = db()->prepare('SELECT * FROM cover_sets WHERE organization_id IS NULL AND user_id = ? ORDER BY is_default DESC, name');
        $stmt->execute([$tenant['user_id']]);
    } else {
        $stmt = db()->prepare('SELECT * FROM cover_sets WHERE organization_id = ? ORDER BY is_default DESC, name');
        $stmt->execute([$tenant['organization_id']]);
    }
    return $stmt->fetchAll();
}

function cover_preview_path(array $tenant, string $storageName): string
{
    if (!preg_match('/^[0-9a-f-]{36}$/', $storageName)) {
        throw new RuntimeException('Identificativo anteprima non valido.');
    }
    return storage_path($tenant['storage'] . "/covers/{$storageName}.png");
}

function cover_preview_base64(array $tenant, ?string $storageName): ?string
{
    if ($storageName === null || $storageName === '') return null;
    $pdf = cover_file_path($tenant, $storageName);
    $png = cover_preview_path($tenant, $storageName);
    if (!is_file($png) && is_file($pdf)) {
        $binary = (string) env_value('EASYPDF_CLI', '/opt/easypdf/bin/easypdf-cli');
        $pipes = [];
        $process = proc_open([$binary, 'render', $pdf, '--page', '1', '--dpi', '24', '--out', $png], [1 => ['pipe', 'w'], 2 => ['pipe', 'w']], $pipes);
        if (is_resource($process)) {
            foreach ($pipes as $pipe) stream_set_blocking($pipe, false);
            $started = microtime(true);
            do {
                $status = proc_get_status($process);
                if (!$status['running']) break;
                if (microtime(true) - $started > 15.0) {
                    proc_terminate($process);
                    break;
                }
                usleep(20_000);
            } while (true);
            foreach ($pipes as $pipe) fclose($pipe);
            proc_close($process);
        }
    }
    if (!is_file($png)) return null;
    $size = filesize($png);
    if (!is_int($size) || $size < 8 || $size > 1024 * 1024) return null;
    $bytes = file_get_contents($png);
    return is_string($bytes) ? base64_encode($bytes) : null;
}

function create_cover_set(array $tenant, string $name, ?array $front, ?array $back, bool $makeDefault): void
{
    if (!$tenant['can_manage']) {
        throw new RuntimeException('Solo gli amministratori dell’organizzazione possono gestire i set cover.');
    }
    $name = trim($name);
    if ($name === '' || mb_strlen($name) > 120) {
        throw new RuntimeException('Inserisci un nome di massimo 120 caratteri.');
    }
    if ($front === null && $back === null) {
        throw new RuntimeException('Carica almeno una cover frontale o posteriore.');
    }
    $directory = storage_path($tenant['storage'] . '/covers');
    ensure_directory($directory);
    $saved = [];
    try {
        $frontId = $front === null ? null : random_id();
        $backId = $back === null ? null : random_id();
        if ($front !== null) {
            $target = cover_file_path($tenant, $frontId);
            if (!move_uploaded_file($front[0], $target)) {
                throw new RuntimeException('Impossibile salvare la cover frontale.');
            }
            $saved[] = $target;
        }
        if ($back !== null) {
            $target = cover_file_path($tenant, $backId);
            if (!move_uploaded_file($back[0], $target)) {
                throw new RuntimeException('Impossibile salvare la cover posteriore.');
            }
            $saved[] = $target;
        }
        $pdo = db();
        $pdo->beginTransaction();
        $lock = $tenant['organization_id'] === null
            ? $pdo->prepare('SELECT id FROM users WHERE id = ? FOR UPDATE')
            : $pdo->prepare('SELECT id FROM organizations WHERE id = ? AND is_active = 1 FOR UPDATE');
        $lock->execute([$tenant['organization_id'] ?? $tenant['user_id']]);
        if (!$lock->fetch()) {
            throw new RuntimeException('Utente non trovato.');
        }
        $sets = user_cover_sets($tenant);
        $makeDefault = $makeDefault || count($sets) === 0;
        if ($makeDefault) {
            clear_default_cover($pdo, $tenant);
        }
        $stmt = $pdo->prepare('INSERT INTO cover_sets (user_id, organization_id, name, front_original_name, front_storage_name, back_original_name, back_storage_name, is_default) VALUES (?, ?, ?, ?, ?, ?, ?, ?)');
        $stmt->execute([$tenant['user_id'], $tenant['organization_id'], $name, $front[1] ?? null, $frontId, $back[1] ?? null, $backId, $makeDefault ? 1 : 0]);
        $pdo->commit();
    } catch (Throwable $error) {
        if (db()->inTransaction()) {
            db()->rollBack();
        }
        foreach ($saved as $path) {
            @unlink($path);
        }
        throw $error;
    }
}

function clear_default_cover(PDO $pdo, array $tenant): void
{
    if ($tenant['organization_id'] === null) {
        $pdo->prepare('UPDATE cover_sets SET is_default = 0 WHERE organization_id IS NULL AND user_id = ?')->execute([$tenant['user_id']]);
    } else {
        $pdo->prepare('UPDATE cover_sets SET is_default = 0 WHERE organization_id = ?')->execute([$tenant['organization_id']]);
    }
}

function cover_set_for_tenant(array $tenant, int $coverSetId, bool $lock = false): array|false
{
    $suffix = $lock ? ' FOR UPDATE' : '';
    if ($tenant['organization_id'] === null) {
        $stmt = db()->prepare('SELECT * FROM cover_sets WHERE id = ? AND organization_id IS NULL AND user_id = ?' . $suffix);
        $stmt->execute([$coverSetId, $tenant['user_id']]);
    } else {
        $stmt = db()->prepare('SELECT * FROM cover_sets WHERE id = ? AND organization_id = ?' . $suffix);
        $stmt->execute([$coverSetId, $tenant['organization_id']]);
    }
    return $stmt->fetch();
}

function set_default_cover(array $tenant, int $coverSetId): void
{
    if (!$tenant['can_manage']) throw new RuntimeException('Operazione riservata agli amministratori dell’organizzazione.');
    $pdo = db();
    $pdo->beginTransaction();
    try {
        if (!cover_set_for_tenant($tenant, $coverSetId, true)) {
            throw new RuntimeException('Set cover non trovato.');
        }
        clear_default_cover($pdo, $tenant);
        $pdo->prepare('UPDATE cover_sets SET is_default = 1 WHERE id = ?')->execute([$coverSetId]);
        $pdo->commit();
    } catch (Throwable $error) {
        $pdo->rollBack();
        throw $error;
    }
}

function delete_cover_set(array $tenant, int $coverSetId): void
{
    if (!$tenant['can_manage']) throw new RuntimeException('Operazione riservata agli amministratori dell’organizzazione.');
    $pdo = db();
    $pdo->beginTransaction();
    try {
        $set = cover_set_for_tenant($tenant, $coverSetId, true);
        if (!$set) {
            throw new RuntimeException('Set cover non trovato.');
        }
        $pdo->prepare('DELETE FROM cover_sets WHERE id = ?')->execute([$coverSetId]);
        if ((bool) $set['is_default']) {
            $remaining = user_cover_sets($tenant);
            $nextId = $remaining[0]['id'] ?? false;
            if ($nextId !== false) {
                $pdo->prepare('UPDATE cover_sets SET is_default = 1 WHERE id = ?')->execute([(int) $nextId]);
            }
        }
        $pdo->commit();
    } catch (Throwable $error) {
        if ($pdo->inTransaction()) {
            $pdo->rollBack();
        }
        throw $error;
    }
    foreach (['front_storage_name', 'back_storage_name'] as $key) {
        if (is_string($set[$key]) && $set[$key] !== '') {
            @unlink(cover_file_path($tenant, $set[$key]));
            @unlink(cover_preview_path($tenant, $set[$key]));
        }
    }
}

function selected_cover_set(array $tenant, ?int $requestedId): array
{
    if ($requestedId !== null) {
        $set = cover_set_for_tenant($tenant, $requestedId);
    } else {
        $set = array_values(array_filter(user_cover_sets($tenant), static fn(array $item): bool => (bool) $item['is_default']))[0] ?? false;
    }
    if (!$set) {
        throw new RuntimeException('Seleziona un set cover o imposta un set predefinito.');
    }
    return $set;
}

function apply_cover_set(array $tenant, array $set, array $uploadedPdf): array
{
    [$inputTmp, $originalName] = validate_pdf_upload($uploadedPdf, 'Documento');
    $jobId = random_id();
    $job = storage_path('jobs/' . $tenant['storage'] . "/{$tenant['user_id']}/{$jobId}");
    ensure_directory($job);
    try {
        $input = $job . '/input.pdf';
        $output = $job . '/output.pdf';
        if (!move_uploaded_file($inputTmp, $input)) {
            throw new RuntimeException('Impossibile preparare il documento.');
        }
        $args = ['doc' => 1];
        foreach (['front', 'back'] as $position) {
            $key = $position . '_storage_name';
            if (is_string($set[$key]) && $set[$key] !== '') {
                $source = cover_file_path($tenant, $set[$key]);
                $targetName = $position . '.pdf';
                if (!is_file($source) || !copy($source, $job . '/' . $targetName)) {
                    throw new RuntimeException("La cover {$position} non è disponibile.");
                }
                $args[$position] = $targetName;
            }
        }
        $script = [
            ['tool' => 'doc_open', 'args' => ['path' => 'input.pdf']],
            ['tool' => 'doc_apply_covers', 'args' => $args],
            ['tool' => 'doc_save', 'args' => ['doc' => 1, 'path' => 'output.pdf']],
        ];
        $scriptPath = $job . '/apply-cover.json';
        file_put_contents($scriptPath, json_encode($script, JSON_PRETTY_PRINT | JSON_THROW_ON_ERROR));
        $binary = (string) env_value('EASYPDF_CLI', '/opt/easypdf/bin/easypdf-cli');
        $command = [$binary, 'run', '--script', $scriptPath, '--root', $job, '--report'];
        $pipes = [];
        $process = proc_open($command, [1 => ['pipe', 'w'], 2 => ['pipe', 'w']], $pipes, $job, ['RUST_BACKTRACE' => '0']);
        if (!is_resource($process)) {
            throw new RuntimeException('Motore PDF non disponibile.');
        }
        $stdout = stream_get_contents($pipes[1]);
        $stderr = stream_get_contents($pipes[2]);
        fclose($pipes[1]);
        fclose($pipes[2]);
        $status = proc_close($process);
        if ($status !== 0 || !is_file($output)) {
            audit($tenant['user_id'], 'cover_apply_failed', ['set_id' => (int) $set['id'], 'engine_status' => $status], $tenant['organization_id']);
            throw new RuntimeException('Il motore PDF non ha completato l’operazione. ' . trim((string) $stderr));
        }
        $downloadName = pathinfo($originalName, PATHINFO_FILENAME) . '-con-cover.pdf';
        audit($tenant['user_id'], 'cover_applied', ['set_id' => (int) $set['id'], 'bytes' => filesize($output), 'engine' => json_decode((string) $stdout, true)], $tenant['organization_id']);
        return [$output, $downloadName, $job];
    } catch (Throwable $error) {
        remove_tree($job);
        throw $error;
    }
}
