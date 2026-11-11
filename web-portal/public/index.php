<?php
declare(strict_types=1);
require __DIR__ . '/../src/bootstrap.php';
security_headers(true); // Load only the same-origin mobile navigation script.

function e(string $value): string { return htmlspecialchars($value, ENT_QUOTES | ENT_SUBSTITUTE, 'UTF-8'); }
function site_url(string $path): string { return rtrim((string) env_value('APP_URL', ''), '/') . $path; }
function download_url(string $name, string $fallback): string
{
    $candidate = trim((string) env_value($name, ''));
    if ($candidate === '') return $fallback;
    $scheme = parse_url($candidate, PHP_URL_SCHEME);
    return in_array($scheme, ['http', 'https'], true) ? $candidate : $fallback;
}

try {
    $action = (string) ($_GET['action'] ?? '');
    if ($_SERVER['REQUEST_METHOD'] === 'POST') {
        verify_csrf();
        if ($action === 'logout') {
            $user = current_user();
            if ($user !== null) audit((int) $user['id'], 'logout');
            $_SESSION = [];
            session_regenerate_id(true);
            header('Location: /'); exit;
        }
        if ($action === 'login') {
            $email = mb_strtolower(trim((string) ($_POST['email'] ?? '')));
            $password = (string) ($_POST['password'] ?? '');
            $attempts = (int) ($_SESSION['login_attempts'] ?? 0);
            if ($attempts >= 8 && time() - (int) ($_SESSION['login_last_attempt'] ?? 0) < 900) throw new RuntimeException('Troppi tentativi. Riprova tra 15 minuti.');
            $stmt = db()->prepare('SELECT u.*, o.is_active AS organization_active FROM users u LEFT JOIN organizations o ON o.id = u.organization_id WHERE u.email = ? LIMIT 1');
            $stmt->execute([$email]);
            $candidate = $stmt->fetch();
            $validPassword = $candidate && password_verify($password, (string) $candidate['password_hash']);
            if ($validPassword && ($candidate['approval_status'] ?? 'approved') === 'pending') throw new RuntimeException('Registrazione in attesa di approvazione dell’amministratore.');
            $organizationInactive = $candidate && $candidate['organization_id'] !== null && !(bool) $candidate['organization_active'];
            if (!$candidate || !$validPassword || !(bool) $candidate['is_active'] || ($candidate['approval_status'] ?? 'approved') !== 'approved' || $organizationInactive) {
                $_SESSION['login_attempts'] = $attempts + 1; $_SESSION['login_last_attempt'] = time();
                audit($candidate ? (int) $candidate['id'] : null, 'login_failed');
                throw new RuntimeException('Credenziali non valide.');
            }
            session_regenerate_id(true); $_SESSION['user_id'] = (int) $candidate['id'];
            unset($_SESSION['login_attempts'], $_SESSION['login_last_attempt']);
            audit((int) $candidate['id'], 'login_succeeded');
            header('Location: /editor/'); exit;
        }
        if ($action === 'register') {
            $name = trim((string) ($_POST['display_name'] ?? ''));
            $email = mb_strtolower(trim((string) ($_POST['email'] ?? '')));
            $password = (string) ($_POST['password'] ?? '');
            if (mb_strlen($name) < 2 || mb_strlen($name) > 120) throw new RuntimeException('Inserisci un nome da 2 a 120 caratteri.');
            if (!filter_var($email, FILTER_VALIDATE_EMAIL)) throw new RuntimeException('Inserisci un indirizzo email valido.');
            if (strlen($password) < 8) throw new RuntimeException('La password deve contenere almeno 8 caratteri.');
            if (!hash_equals($password, (string) ($_POST['password_confirm'] ?? ''))) throw new RuntimeException('Le password non coincidono.');
            $stmt = db()->prepare("INSERT INTO users (email, display_name, password_hash, role, is_active, approval_status) VALUES (?, ?, ?, 'user', 0, 'pending')");
            $stmt->execute([$email, $name, password_hash($password, PASSWORD_DEFAULT)]);
            audit((int) db()->lastInsertId(), 'registration_requested');
            flash('success', 'Registrazione inviata. Potrai accedere dopo l’approvazione dell’amministratore.');
            header('Location: /?page=login'); exit;
        }
        if ($action === 'forgot') {
            $email = mb_strtolower(trim((string) ($_POST['email'] ?? '')));
            $stmt = db()->prepare("SELECT id, email FROM users WHERE email = ? AND is_active = 1 AND approval_status = 'approved' LIMIT 1");
            $stmt->execute([$email]); $candidate = $stmt->fetch();
            if ($candidate) {
                $token = rtrim(strtr(base64_encode(random_bytes(32)), '+/', '-_'), '=');
                db()->prepare('DELETE FROM password_reset_tokens WHERE user_id = ? OR expires_at <= CURRENT_TIMESTAMP')->execute([(int) $candidate['id']]);
                db()->prepare('INSERT INTO password_reset_tokens (token_hash, user_id, expires_at) VALUES (?, ?, DATE_ADD(CURRENT_TIMESTAMP, INTERVAL 30 MINUTE))')->execute([hash('sha256', $token), (int) $candidate['id']]);
                $link = site_url('/?page=reset&token=' . rawurlencode($token));
                @mail((string) $candidate['email'], 'Reimposta la password easyPDF', "Apri questo collegamento entro 30 minuti:\n{$link}", 'From: ' . env_value('MAIL_FROM', 'noreply@localhost'));
                if (env_value('APP_ENV', 'production') !== 'production') $_SESSION['development_reset_link'] = $link;
                audit((int) $candidate['id'], 'password_reset_requested');
            }
            flash('success', 'Se l’account esiste, riceverai un collegamento valido per 30 minuti.');
            header('Location: /?page=forgot'); exit;
        }
        if ($action === 'reset') {
            $token = (string) ($_POST['token'] ?? ''); $password = (string) ($_POST['password'] ?? '');
            if (!preg_match('/^[A-Za-z0-9_-]{40,100}$/', $token)) throw new RuntimeException('Collegamento di recupero non valido.');
            if (strlen($password) < 8) throw new RuntimeException('La password deve contenere almeno 8 caratteri.');
            if (!hash_equals($password, (string) ($_POST['password_confirm'] ?? ''))) throw new RuntimeException('Le password non coincidono.');
            $pdo = db(); $pdo->beginTransaction();
            $stmt = $pdo->prepare('SELECT user_id FROM password_reset_tokens WHERE token_hash = ? AND expires_at > CURRENT_TIMESTAMP FOR UPDATE');
            $stmt->execute([hash('sha256', $token)]); $userId = $stmt->fetchColumn();
            if ($userId === false) { $pdo->rollBack(); throw new RuntimeException('Collegamento scaduto o già utilizzato.'); }
            $pdo->prepare('UPDATE users SET password_hash = ? WHERE id = ?')->execute([password_hash($password, PASSWORD_DEFAULT), (int) $userId]);
            $pdo->prepare('DELETE FROM password_reset_tokens WHERE user_id = ?')->execute([(int) $userId]);
            $pdo->prepare('DELETE FROM api_tokens WHERE user_id = ?')->execute([(int) $userId]);
            $pdo->commit(); audit((int) $userId, 'password_reset_completed');
            flash('success', 'Password aggiornata. Ora puoi accedere.'); header('Location: /?page=login'); exit;
        }
        throw new RuntimeException('Operazione non riconosciuta.');
    }
} catch (Throwable $error) {
    try {
        $connection = db();
        if ($connection->inTransaction()) $connection->rollBack();
    } catch (Throwable $rollbackError) {
        error_log('easyPDF rollback failed: ' . $rollbackError->getMessage());
    }
    flash('error', $error instanceof PDOException ? 'Operazione non riuscita. Verifica che l’email non sia già registrata.' : $error->getMessage());
    $target = in_array($action, ['register', 'forgot', 'reset'], true) ? $action : 'login';
    header('Location: /?page=' . rawurlencode($target)); exit;
}

$user = current_user();
$showTopbar = $user === null || is_account_admin($user);
$page = (string) ($_GET['page'] ?? 'home');
if (!in_array($page, ['home', 'login', 'register', 'forgot', 'reset'], true)) $page = 'home';
$flashes = take_flashes();
$resetToken = $page === 'reset' ? (string) ($_GET['token'] ?? '') : '';
$developmentResetLink = isset($_SESSION['development_reset_link']) ? (string) $_SESSION['development_reset_link'] : null;
unset($_SESSION['development_reset_link']);
$repository = 'https://github.com/LFA-Asteriti/easyPDF';
$release = $repository . '/releases';
$downloads = [
    ['Windows', download_url('DOWNLOAD_WINDOWS_URL', $release), 'Windows 10/11 · x64', '/assets/platform/Windows_11.svg', 'blue'],
    ['macOS', download_url('DOWNLOAD_MACOS_URL', $release), 'Apple Silicon e Intel', '/assets/platform/Apple.svg', 'pink'],
    ['Linux', download_url('DOWNLOAD_LINUX_URL', $release), 'x86_64 · archivio tar.gz', '/assets/platform/Linux.svg', 'green'],
];
$macroFeatures = [
    ['edit', 'Modifica un PDF', 'Modifica testo, immagini, pagine e collegamenti.', 'file-pen-line', 'pink'],
    ['organize', 'Organizza pagine', 'Riordina, ruota, inserisce, estrae e divide pagine.', 'layout-grid', 'green'],
    ['export', 'Converti ed esporta', 'Esporta il documento nei formati disponibili.', 'file-output', 'blue'],
    ['combine', 'Combina file', 'Riunisce più documenti in un unico PDF ordinato.', 'files', 'blue'],
    ['ocr', 'Scansione e OCR', 'Riconosce il testo contenuto in scansioni e immagini.', 'scan-text', 'green'],
    ['compress', 'Comprimi un PDF', 'Riduce le dimensioni con controllo sull’ottimizzazione.', 'file-down', 'red'],
    ['protect', 'Proteggi un PDF', 'Gestisce password, permessi e informazioni sensibili.', 'shield-check', 'blue'],
    ['fill-sign', 'Compila e firma', 'Aggiunge compilazioni e firme visive al documento.', 'pen-line', 'purple'],
    ['covers', 'Applica cover', 'Applica copertine singole o kit personali e condivisi.', 'book-open', 'pink'],
    ['forms', 'Prepara un modulo', 'Crea e configura campi modulo interattivi.', 'text-cursor-input', 'purple'],
    ['comments', 'Commenti e revisione', 'Annota, evidenzia e collabora senza alterare il testo.', 'message-square-text', 'orange'],
    ['ai', 'Assistente AI', 'Riepiloga, interroga e traduce con il provider configurato.', 'sparkles', 'pink'],
];
?><!doctype html><html lang="it"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="description" content="easyPDF: editor PDF open source per desktop e web, privato, programmabile e adatto alle organizzazioni.">
<title><?= $page === 'home' ? 'easyPDF · Ogni strumento PDF, un solo spazio' : 'Account · easyPDF' ?></title>
<link rel="icon" href="/assets/favicon.ico"><link rel="stylesheet" href="/app.css?v=20261105-2"><script src="/site-navigation.js?v=20261105-1" defer></script></head><body class="<?= $page === 'home' ? 'home-page' : 'account-page' ?>">
<?php if ($showTopbar): ?><header class="topbar"><a class="brand" href="/"><img src="/assets/easyPDF-logo-h.svg" alt="easyPDF"></a><button class="mobile-menu-button site-menu-button" type="button" aria-label="Menu principale" aria-controls="site-navigation" aria-expanded="false" data-menu-toggle><span class="menu-hamburger" aria-hidden="true"></span></button><nav id="site-navigation" class="site-navigation" aria-label="Navigazione principale"><a href="/#funzioni">Funzioni</a><a href="/documentazione/">Documentazione</a><a href="/#download">Download</a>
<?php if ($user): ?><a href="/supporto/">Supporto</a><a class="button small" href="/editor/">Apri easyPDF</a><div class="account-menu"><div class="account-trigger"><a href="/editor/?preferences=account">Account</a><button type="button" aria-label="Apri il menu account" aria-haspopup="true">⌄</button></div><div class="account-submenu"><form method="post" action="/?action=logout"><input type="hidden" name="csrf" value="<?= e(csrf_token()) ?>"><button class="account-logout"><span class="menu-icon" aria-hidden="true"></span>Esci</button></form></div></div>
<?php else: ?><a href="/?page=login">Accedi</a><a class="button small" href="/?page=register">Registrati</a><?php endif; ?></nav></header><?php endif; ?>
<main class="shell public-shell"><?php foreach ($flashes as $flash): ?><div class="notice <?= e((string) $flash['type']) ?>" role="status"><?= e((string) $flash['message']) ?></div><?php endforeach; ?>
<?php if ($developmentResetLink): ?><div class="notice success">Solo sviluppo: <a href="<?= e($developmentResetLink) ?>">apri il recupero password</a>.</div><?php endif; ?>
<?php if ($page === 'home'): ?>
<section class="home-hero">
    <div class="home-hero-copy">
        <span class="eyebrow">PDF, senza complicazioni</span>
        <h1>Ogni strumento PDF.<br><em>Un solo spazio.</em></h1>
        <p>Leggi, modifica, converti, firma, proteggi e automatizza i tuoi PDF con la stessa esperienza sul web e sul desktop.</p>
        <div class="hero-actions">
            <a class="primary home-primary" href="<?= $user ? '/editor/' : '/?page=register' ?>"><span class="site-icon icon-app-window" aria-hidden="true"></span><?= $user ? 'Apri easyPDF' : 'Inizia con easyPDF' ?></a>
            <a class="button ghost home-secondary" href="#download"><span class="site-icon icon-file-down" aria-hidden="true"></span>Scarica l’app</a>
        </div>
        <div class="platform-list" aria-label="Piattaforme disponibili">
            <span><i class="site-icon icon-globe" aria-hidden="true"></i> Web</span>
            <span><img class="platform-logo" src="/assets/platform/Windows_11.svg" alt="" aria-hidden="true"> Windows</span>
            <span><img class="platform-logo" src="/assets/platform/Apple.svg" alt="" aria-hidden="true"> macOS</span>
            <span><img class="platform-logo" src="/assets/platform/Linux.svg" alt="" aria-hidden="true"> Linux</span>
        </div>
    </div>
    <div class="app-showcase hero-showcase" aria-label="Anteprima dell’editor easyPDF">
        <div class="showcase-orbit orbit-one"></div><div class="showcase-orbit orbit-two"></div>
        <div class="app-frame screenshot-frame">
            <img src="/assets/home/easyPDF-home-1.jpg" width="1352" height="910" alt="Editor easyPDF con un documento aperto" fetchpriority="high">
        </div>
        <p class="shot-caption"><span class="site-icon icon-camera" aria-hidden="true"></span>easyPDF in azione</p>
    </div>
</section>

<section class="home-proof" aria-label="Caratteristiche principali">
    <span><i class="site-icon icon-lock" aria-hidden="true"></i><b>Locale e privato</b><small>I PDF restano sul dispositivo</small></span>
    <span><i class="site-icon icon-monitor" aria-hidden="true"></i><b>Web + desktop</b><small>Un’interfaccia coerente</small></span>
    <span><i class="site-icon icon-user-round" aria-hidden="true"></i><b>Per organizzazioni</b><small>Account e kit condivisi</small></span>
    <span><i class="site-icon icon-square-terminal" aria-hidden="true"></i><b>Automazione opt-in</b><small>CLI e strumenti autorizzati</small></span>
</section>

<section id="funzioni" class="home-tools">
    <header class="home-section-head">
        <div><span class="eyebrow">Tutto quello che serve</span><h2>Le macro-funzioni di easyPDF</h2></div>
        <p>Scegli uno strumento per aprire direttamente la relativa sezione della documentazione.</p>
    </header>
    <div class="macro-grid">
        <?php foreach ($macroFeatures as [$id, $title, $description, $icon, $tone]): ?>
            <a class="macro-card tone-<?= e($tone) ?>" href="/documentazione/#<?= e($id) ?>">
                <span class="site-icon macro-icon icon-<?= e($icon) ?>" aria-hidden="true"></span><strong><?= e($title) ?></strong><span><?= e($description) ?></span>
                <b>Apri la guida <i class="site-icon icon-chevron-right" aria-hidden="true"></i></b>
            </a>
        <?php endforeach; ?>
    </div>
    <a class="all-tools-link" href="/documentazione/"><span class="site-icon icon-layout-grid" aria-hidden="true"></span>Esplora tutte le funzionalità documentate</a>
</section>

<section class="home-story" aria-label="Scopri easyPDF">
    <article class="story-row">
        <div class="story-copy"><span class="eyebrow">Modifica e revisione</span><h2>Dal testo ai commenti, senza cambiare applicazione.</h2><p>Intervieni su contenuti e pagine, aggiungi annotazioni, compila e firma. Gli strumenti sono organizzati nello stesso modo sul web e sul desktop.</p><div class="story-links"><a href="/documentazione/#edit">Modifica PDF</a><a href="/documentazione/#comments">Commenti e revisione</a><a href="/documentazione/#fill-sign">Compila e firma</a></div></div>
        <figure class="story-screenshot tone-pink"><img src="/assets/home/easyPDF-home-2.jpg" width="1240" height="854" loading="lazy" alt="Strumenti di modifica e revisione di easyPDF"></figure>
    </article>
    <article class="story-row story-reverse">
        <div class="story-copy"><span class="eyebrow">Ordine e leggerezza</span><h2>Organizza, combina e ottimizza anche documenti complessi.</h2><p>Lavora sulle miniature, riordina le pagine, unisci più file e riduci le dimensioni scegliendo il livello di controllo più adatto.</p><div class="story-links"><a href="/documentazione/#organize">Organizza pagine</a><a href="/documentazione/#combine">Combina file</a><a href="/documentazione/#compress">Comprimi PDF</a></div></div>
        <figure class="story-screenshot tone-green"><img src="/assets/home/easyPDF-home-3.jpg" width="1241" height="851" loading="lazy" alt="Organizzazione delle pagine di un PDF in easyPDF"></figure>
    </article>
    <article class="story-row">
        <div class="story-copy"><span class="eyebrow">Squadre e intelligenza artificiale</span><h2>Kit cover, account e provider AI sotto il tuo controllo.</h2><p>Collega l’app desktop alla tua installazione web, sincronizza i kit autorizzati e scegli se usare un provider AI personale o condiviso dall’organizzazione.</p><div class="story-links"><a href="/documentazione/#covers">Kit cover</a><a href="/documentazione/#ai">Assistente AI</a><a href="/editor/?preferences=account">Account e organizzazione</a></div></div>
        <figure class="story-screenshot tone-purple"><img src="/assets/home/easyPDF-home-4.jpg" width="1241" height="851" loading="lazy" alt="Impostazioni e strumenti dell’editor easyPDF"></figure>
    </article>
</section>

<section class="home-privacy">
    <div class="privacy-mark"><span class="site-icon icon-shield-check" aria-hidden="true"></span></div>
    <div><span class="eyebrow">Privacy per impostazione</span><h2>Il documento resta sul dispositivo.</h2><p>L’editor elabora i PDF localmente. La sincronizzazione riguarda soltanto account, organizzazioni, preferenze e kit autorizzati.</p></div>
    <a href="/documentazione/#protect">Scopri gli strumenti di protezione <span aria-hidden="true">→</span></a>
</section>

<section id="download" class="home-download">
    <header class="home-section-head"><div><span class="eyebrow">Web o desktop</span><h2>Lavora dove preferisci.</h2></div><p>Apri subito l’editor web oppure consulta le versioni desktop disponibili nelle release GitHub.</p></header>
    <div class="download-grid home-download-grid">
        <?php foreach ($downloads as [$label, $url, $note, $iconUrl, $tone]): ?>
            <a class="download-card tone-<?= e($tone) ?>" href="<?= e($url) ?>"><img class="download-icon download-icon-image" src="<?= e($iconUrl) ?>" alt="" aria-hidden="true"><strong><?= e($label) ?></strong><span><?= e($note) ?></span><b><i class="site-icon icon-file-down" aria-hidden="true"></i>Versioni per <?= e($label) ?></b></a>
        <?php endforeach; ?>
        <a class="download-card web-download tone-blue" href="<?= $user ? '/editor/' : '/?page=login' ?>"><span class="site-icon download-icon icon-globe" aria-hidden="true"></span><strong>Web</strong><span>Nessuna installazione richiesta</span><b><i class="site-icon icon-arrow-up-right" aria-hidden="true"></i>Apri l’editor web</b></a>
    </div>
    <div class="story-links"><a href="<?= e($release) ?>">Tutte le versioni e le note di rilascio</a></div>
</section>

<section class="home-final-cta">
    <span class="site-icon icon-file-check" aria-hidden="true"></span><div><span class="eyebrow">Pronto per iniziare?</span><h2>Porta ogni attività PDF in un unico flusso di lavoro.</h2></div>
    <div><a class="primary" href="<?= $user ? '/editor/' : '/?page=register' ?>"><?= $user ? 'Apri easyPDF' : 'Crea il tuo account' ?></a><a class="button ghost" href="/documentazione/">Leggi la documentazione</a></div>
</section>
<?php else: ?><section class="login-card card"><img class="login-logo" src="/assets/easyPDF-logo.svg" alt="easyPDF">
<?php if ($page === 'login'): ?><h1>Accedi</h1><p>Apri il tuo spazio easyPDF e sincronizza account e kit cover.</p><form method="post" action="/?action=login" class="stack"><input type="hidden" name="csrf" value="<?= e(csrf_token()) ?>"><label>Email<input type="email" name="email" autocomplete="username" required></label><label>Password<input type="password" name="password" autocomplete="current-password" required></label><button class="primary">Accedi</button></form><p class="form-links"><a href="/?page=forgot">Password dimenticata?</a> · <a href="/?page=register">Registrati</a></p>
<?php elseif ($page === 'register'): ?><h1>Registrati</h1><p>La richiesta sarà verificata dall’amministratore.</p><form method="post" action="/?action=register" class="stack"><input type="hidden" name="csrf" value="<?= e(csrf_token()) ?>"><label>Nome e cognome<input name="display_name" maxlength="120" required></label><label>Email<input type="email" name="email" maxlength="254" required></label><label>Password <span>almeno 8 caratteri</span><input type="password" name="password" minlength="8" required></label><label>Ripeti password<input type="password" name="password_confirm" minlength="8" required></label><button class="primary">Invia registrazione</button></form><p class="form-links"><a href="/?page=login">Hai già un account?</a></p>
<?php elseif ($page === 'forgot'): ?><h1>Recupera l’accesso</h1><p>Riceverai un collegamento monouso valido per 30 minuti.</p><form method="post" action="/?action=forgot" class="stack"><input type="hidden" name="csrf" value="<?= e(csrf_token()) ?>"><label>Email<input type="email" name="email" required></label><button class="primary">Invia collegamento</button></form><p class="form-links"><a href="/?page=login">Torna all’accesso</a></p>
<?php else: ?><h1>Nuova password</h1><p>Scegli una password di almeno 8 caratteri.</p><form method="post" action="/?action=reset" class="stack"><input type="hidden" name="csrf" value="<?= e(csrf_token()) ?>"><input type="hidden" name="token" value="<?= e($resetToken) ?>"><label>Nuova password<input type="password" name="password" minlength="8" required></label><label>Ripeti password<input type="password" name="password_confirm" minlength="8" required></label><button class="primary">Aggiorna password</button></form><?php endif; ?></section><?php endif; ?></main>
<footer>easyPDF by Massimo Bertolucci · <a href="<?= e($repository) ?>">Codice sorgente</a></footer></body></html>
