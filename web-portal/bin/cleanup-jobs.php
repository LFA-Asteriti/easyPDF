<?php
declare(strict_types=1);

require_once __DIR__ . '/../src/bootstrap.php';

if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

$jobsRoot = storage_path('jobs');
$cutoff = time() - 3600;
$removed = 0;
if (is_dir($jobsRoot)) {
    $users = new DirectoryIterator($jobsRoot);
    foreach ($users as $userDirectory) {
        if ($userDirectory->isDot() || !$userDirectory->isDir()) continue;
        foreach (new DirectoryIterator($userDirectory->getPathname()) as $jobDirectory) {
            if ($jobDirectory->isDot() || !$jobDirectory->isDir()) continue;
            if ($jobDirectory->getMTime() < $cutoff) {
                remove_tree($jobDirectory->getPathname());
                $removed++;
            }
        }
    }
}
fwrite(STDOUT, "Job rimossi: {$removed}\n");
