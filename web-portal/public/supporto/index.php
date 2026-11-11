<?php
declare(strict_types=1);

require dirname(__DIR__, 2) . '/src/bootstrap.php';
security_headers(true); // Load only the same-origin mobile navigation script.

function e(string $value): string
{
    return htmlspecialchars($value, ENT_QUOTES | ENT_SUBSTITUTE, 'UTF-8');
}

function support_text(string $name, int $min, int $max): string
{
    $value = trim((string) ($_POST[$name] ?? ''));
    $length = mb_strlen($value);
    if ($length < $min || $length > $max) {
        throw new RuntimeException("Il campo {$name} deve contenere da {$min} a {$max} caratteri.");
    }
    return $value;
}

function support_send(string $recipient, string $subject, string $body, array $user): string
{
    $from = trim((string) env_value('MAIL_FROM', 'noreply@localhost'));
    if (!filter_var($from, FILTER_VALIDATE_EMAIL)) {
        $from = 'noreply@localhost';
    }
    $headers = [
        'From: easyPDF <' . $from . '>',
        'Reply-To: ' . (string) $user['email'],
        'Content-Type: text/plain; charset=UTF-8',
        'X-Mailer: easyPDF Support',
    ];
    if (@mail($recipient, mb_encode_mimeheader($subject, 'UTF-8'), $body, implode("\r\n", $headers))) {
        return 'sent';
    }
    if (env_value('APP_ENV', 'production') !== 'production') {
        $directory = storage_path('support-outbox');
        ensure_directory($directory);
        $path = $directory . DIRECTORY_SEPARATOR . random_id() . '.eml';
        $message = implode("\r\n", ['To: ' . $recipient, 'Subject: ' . $subject, ...$headers, '', $body]);
        if (file_put_contents($path, $message, LOCK_EX) !== false) {
            return 'queued';
        }
    }
    throw new RuntimeException('Invio non riuscito. Contatta il Super Admin per verificare la configurazione email del server.');
}

$user = require_user();
$categories = [
    'funzione' => 'Problema con una funzionalità',
    'account' => 'Account e accesso',
    'organizzazione' => 'Organizzazione e sincronizzazione',
    'cover' => 'Kit cover',
    'ai' => 'Provider AI',
    'errore' => 'Errore o arresto anomalo',
    'suggerimento' => 'Suggerimento',
    'altro' => 'Altro',
];
$priorities = ['bassa' => 'Bassa', 'normale' => 'Normale', 'alta' => 'Alta', 'bloccante' => 'Bloccante'];

try {
    if ($_SERVER['REQUEST_METHOD'] === 'POST') {
        verify_csrf();
        $action = (string) ($_POST['action'] ?? 'message');
        if ($action === 'settings') {
            if ($user['role'] !== 'super_admin') {
                throw new RuntimeException('Operazione riservata al Super Admin.');
            }
            $email = mb_strtolower(trim((string) ($_POST['support_email'] ?? '')));
            if (!filter_var($email, FILTER_VALIDATE_EMAIL) || mb_strlen($email) > 254) {
                throw new RuntimeException('Inserisci un indirizzo email di supporto valido.');
            }
            save_site_setting('support_email', $email, (int) $user['id']);
            audit((int) $user['id'], 'support_email_changed');
            flash('success', 'Indirizzo email del supporto aggiornato.');
            header('Location: /supporto/');
            exit;
        }
        if ($action !== 'message') {
            throw new RuntimeException('Operazione non riconosciuta.');
        }
        $lastMessage = (int) ($_SESSION['support_last_message'] ?? 0);
        if ($lastMessage > 0 && time() - $lastMessage < 60) {
            throw new RuntimeException('Attendi un minuto prima di inviare un altro messaggio.');
        }
        $recipient = support_email();
        if ($recipient === null) {
            throw new RuntimeException('Il Super Admin non ha ancora configurato l’indirizzo email del supporto.');
        }
        $category = (string) ($_POST['category'] ?? '');
        $priority = (string) ($_POST['priority'] ?? 'normale');
        if (!isset($categories[$category]) || !isset($priorities[$priority])) {
            throw new RuntimeException('Categoria o priorità non valida.');
        }
        $subjectText = support_text('subject', 4, 140);
        $message = support_text('message', 20, 6000);
        $steps = trim((string) ($_POST['steps'] ?? ''));
        $expected = trim((string) ($_POST['expected'] ?? ''));
        $environment = trim((string) ($_POST['environment'] ?? ''));
        if (mb_strlen($steps) > 3000 || mb_strlen($expected) > 2000 || mb_strlen($environment) > 500) {
            throw new RuntimeException('Uno dei campi di dettaglio è troppo lungo.');
        }
        $safeSubject = str_replace(["\r", "\n"], ' ', $subjectText);
        $mailSubject = '[easyPDF][' . $priorities[$priority] . '] ' . $safeSubject;
        $organization = trim((string) ($user['organization_name'] ?? ''));
        $body = implode("\n", [
            'NUOVA RICHIESTA DI SUPPORTO EASYPDF',
            '',
            'Utente: ' . (string) $user['display_name'],
            'Email account: ' . (string) $user['email'],
            'Organizzazione: ' . ($organization === '' ? 'Account personale' : $organization),
            'Categoria: ' . $categories[$category],
            'Priorità: ' . $priorities[$priority],
            'Oggetto: ' . $safeSubject,
            '',
            'DESCRIZIONE',
            $message,
            '',
            'PASSAGGI PER RIPRODURRE',
            $steps === '' ? 'Non indicati.' : $steps,
            '',
            'RISULTATO ATTESO',
            $expected === '' ? 'Non indicato.' : $expected,
            '',
            'AMBIENTE',
            $environment === '' ? 'Non indicato.' : $environment,
        ]);
        $delivery = support_send($recipient, $mailSubject, $body, $user);
        $_SESSION['support_last_message'] = time();
        audit((int) $user['id'], 'support_message_' . $delivery, ['category' => $category, 'priority' => $priority]);
        flash('success', $delivery === 'sent' ? 'Messaggio inviato al supporto.' : 'Server email locale non disponibile: messaggio salvato nell’outbox di sviluppo.');
        header('Location: /supporto/');
        exit;
    }
} catch (Throwable $error) {
    flash('error', $error->getMessage());
    header('Location: /supporto/');
    exit;
}

$recipient = support_email();
$flashes = take_flashes();
?><!doctype html>
<html lang="it">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="description" content="Contatta il supporto easyPDF dal tuo account autorizzato.">
    <title>Supporto · easyPDF</title>
    <link rel="icon" href="/assets/favicon.ico">
    <link rel="stylesheet" href="/app.css?v=20261105-2">
    <script src="/site-navigation.js?v=20261105-1" defer></script>
</head>
<body>
<header class="topbar">
    <a class="brand" href="/"><img src="/assets/easyPDF-logo-h.svg" alt="easyPDF"></a>
    <button class="mobile-menu-button site-menu-button" type="button" aria-label="Menu principale" aria-controls="site-navigation" aria-expanded="false" data-menu-toggle><span class="menu-hamburger" aria-hidden="true"></span></button>
    <nav id="site-navigation" class="site-navigation" aria-label="Navigazione principale">
        <a href="/">Home</a>
        <a href="/documentazione/">Documentazione</a>
        <a href="/supporto/" aria-current="page">Supporto</a>
        <a class="button small" href="/editor/">Apri easyPDF</a>
        <div class="account-menu">
            <div class="account-trigger"><a href="/editor/?preferences=account">Account</a><button type="button" aria-label="Apri il menu account" aria-haspopup="true">⌄</button></div>
            <div class="account-submenu"><form method="post" action="/?action=logout"><input type="hidden" name="csrf" value="<?= e(csrf_token()) ?>"><button class="account-logout"><span class="menu-icon" aria-hidden="true"></span>Esci</button></form></div>
        </div>
    </nav>
</header>
<main class="shell support-shell">
    <?php foreach ($flashes as $flash): ?>
        <div class="notice <?= e((string) $flash['type']) ?>" role="status"><?= e((string) $flash['message']) ?></div>
    <?php endforeach; ?>
    <header class="page-head">
        <div><span class="eyebrow">Area riservata</span><h1>Supporto easyPDF</h1><p>Descrivi il problema con tutti i dettagli utili. Nome, email dell’account e organizzazione vengono inseriti automaticamente.</p></div>
        <span class="privacy-pill">Accessibile solo dopo il login</span>
    </header>
    <div class="grid support-grid">
        <section class="card">
            <h2>Invia un messaggio</h2>
            <?php if ($recipient === null): ?>
                <div class="notice error">Il servizio non è ancora configurato dal Super Admin.</div>
            <?php endif; ?>
            <form method="post" action="/supporto/" class="stack support-form">
                <input type="hidden" name="csrf" value="<?= e(csrf_token()) ?>">
                <input type="hidden" name="action" value="message">
                <div class="form-columns">
                    <label>Nome<input value="<?= e((string) $user['display_name']) ?>" readonly></label>
                    <label>Email account<input value="<?= e((string) $user['email']) ?>" readonly></label>
                </div>
                <div class="form-columns">
                    <label>Categoria<select name="category" required><?php foreach ($categories as $value => $label): ?><option value="<?= e($value) ?>"><?= e($label) ?></option><?php endforeach; ?></select></label>
                    <label>Priorità<select name="priority" required><?php foreach ($priorities as $value => $label): ?><option value="<?= e($value) ?>"<?= $value === 'normale' ? ' selected' : '' ?>><?= e($label) ?></option><?php endforeach; ?></select></label>
                </div>
                <label>Oggetto<input name="subject" minlength="4" maxlength="140" required placeholder="Riassumi il problema in una riga"></label>
                <label>Descrizione dettagliata<textarea name="message" minlength="20" maxlength="6000" rows="7" required placeholder="Cosa è successo? Indica file coinvolto senza inserire dati riservati."></textarea></label>
                <label>Passaggi per riprodurre <span>facoltativo</span><textarea name="steps" maxlength="3000" rows="5" placeholder="1. Apro… 2. Seleziono… 3. Compare…"></textarea></label>
                <label>Risultato atteso <span>facoltativo</span><textarea name="expected" maxlength="2000" rows="3" placeholder="Cosa ti aspettavi che accadesse?"></textarea></label>
                <label>Ambiente <span>facoltativo</span><input name="environment" maxlength="500" placeholder="Web o desktop, sistema operativo, browser e versione"></label>
                <button class="primary"<?= $recipient === null ? ' disabled' : '' ?>>Invia al supporto</button>
            </form>
        </section>
        <aside class="stack">
            <section class="card soft">
                <h2>Prima di inviare</h2>
                <ol><li>Consulta la <a href="/documentazione/">documentazione</a>.</li><li>Non inserire password, chiavi API o dati personali contenuti nei PDF.</li><li>Indica i passaggi esatti che causano il problema.</li></ol>
            </section>
            <?php if ($user['role'] === 'super_admin'): ?>
                <section class="card admin-setting">
                    <span class="badge">Super Admin</span>
                    <h2>Destinatario supporto</h2>
                    <p>Ogni richiesta verrà inviata a questo indirizzo. Il mittente usa la configurazione email del server.</p>
                    <form method="post" action="/supporto/" class="stack">
                        <input type="hidden" name="csrf" value="<?= e(csrf_token()) ?>">
                        <input type="hidden" name="action" value="settings">
                        <label>Email supporto<input type="email" name="support_email" maxlength="254" required value="<?= e($recipient ?? '') ?>" placeholder="supporto@example.it"></label>
                        <button class="button small">Salva indirizzo</button>
                    </form>
                </section>
            <?php endif; ?>
        </aside>
    </div>
</main>
<footer>easyPDF by Massimo Bertolucci · <a href="/documentazione/">Documentazione</a></footer>
</body>
</html>
