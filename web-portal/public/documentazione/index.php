<?php
declare(strict_types=1);

require dirname(__DIR__, 2) . '/src/bootstrap.php';
require dirname(__DIR__, 2) . '/src/documentation.php';

// This page uses only same-origin JavaScript. Inline styles carry the icon name
// and the colour selected from the hard-coded easyPDF tool catalogue below.
header("Content-Security-Policy: default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; form-action 'self'; object-src 'none'; frame-ancestors 'none'; base-uri 'self'");

function e(string $value): string
{
    return htmlspecialchars($value, ENT_QUOTES | ENT_SUBSTITUTE, 'UTF-8');
}

$user = current_user();
$groups = documentation_groups();
$functionCount = 0;
foreach ($groups as $group) {
    foreach ($group['sections'] as $section) {
        $functionCount += count($section['items']);
    }
}
$navigationSections = [
    ['title' => 'Operazioni PDF', 'groups' => array_slice($groups, 0, 12)],
    ['title' => 'Moduli e firme', 'groups' => array_slice($groups, 12, 3)],
    ['title' => 'Analisi e automazione', 'groups' => array_slice($groups, 15)],
];
?><!doctype html>
<html lang="it">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="description" content="Documentazione completa delle funzionalità disponibili in easyPDF per web e desktop.">
    <title>Documentazione · easyPDF</title>
    <link rel="icon" href="/assets/favicon.ico">
    <link rel="stylesheet" href="/app.css?v=20261105-2">
    <link rel="stylesheet" href="/documentazione/documentation.css?v=20261105-1">
    <script src="/site-navigation.js?v=20261105-1" defer></script>
    <script src="/documentazione/documentation.js?v=20261022-3" defer></script>
</head>
<body class="docs-page">
<header class="topbar docs-topbar">
    <button class="mobile-menu-button docs-index-button" type="button" aria-label="Indice della documentazione" aria-controls="docs-index" aria-expanded="false" data-menu-toggle><span class="menu-hamburger" aria-hidden="true"></span></button>
    <a class="brand" href="/"><img src="/assets/easyPDF-logo-h.svg" alt="easyPDF"></a>
    <button class="mobile-menu-button site-menu-button" type="button" aria-label="Menu principale" aria-controls="site-navigation" aria-expanded="false" data-menu-toggle><span class="menu-hamburger" aria-hidden="true"></span></button>
    <nav id="site-navigation" class="site-navigation" aria-label="Navigazione principale">
        <a href="/#funzioni">Funzioni</a>
        <a href="/documentazione/" aria-current="page">Documentazione</a>
        <a href="/#download">Download</a>
        <?php if ($user): ?>
            <a href="/supporto/">Supporto</a>
            <a class="button small" href="/editor/">Apri easyPDF</a>
            <div class="account-menu">
                <div class="account-trigger"><a href="/editor/?preferences=account">Account</a><button type="button" aria-label="Apri il menu account" aria-haspopup="true">⌄</button></div>
                <div class="account-submenu"><form method="post" action="/?action=logout"><input type="hidden" name="csrf" value="<?= e(csrf_token()) ?>"><button class="account-logout"><span class="menu-icon" aria-hidden="true"></span>Esci</button></form></div>
            </div>
        <?php else: ?>
            <a href="/?page=login">Accedi</a>
            <a class="button small" href="/?page=register">Registrati</a>
        <?php endif; ?>
    </nav>
</header>

<div id="top" class="docs-workspace">
    <aside id="docs-index" class="docs-sidebar" aria-label="Indice della documentazione">
        <div class="docs-sidebar-head">
            <span class="docs-product">easyPDF</span>
            <strong>Documentazione</strong>
            <p><?= count($groups) ?> strumenti · <?= $functionCount ?> funzionalità</p>
        </div>
        <label class="docs-search">
            <span class="tool-icon" style="--tool-icon:url('/assets/icons/search.svg')" aria-hidden="true"></span>
            <span class="sr-only">Filtra gli strumenti per nome</span>
            <input id="docs-search" type="search" placeholder="Filtra per nome…" autocomplete="off">
        </label>
        <nav class="docs-navigation" aria-label="Strumenti easyPDF">
            <?php foreach ($navigationSections as $navigationSection): ?>
                <section data-doc-nav-section>
                    <h2><?= e($navigationSection['title']) ?></h2>
                    <?php foreach ($navigationSection['groups'] as $group): ?>
                        <a href="#<?= e($group['id']) ?>" data-doc-link="<?= e($group['id']) ?>">
                            <span class="tool-icon" style="--tool-icon:url('/assets/icons/<?= e($group['icon']) ?>.svg');--tool-color:<?= e($group['color']) ?>" aria-hidden="true"></span>
                            <span><?= e($group['title']) ?></span>
                        </a>
                    <?php endforeach; ?>
                </section>
            <?php endforeach; ?>
        </nav>
        <div class="docs-sidebar-help">
            <span class="tool-icon" style="--tool-icon:url('/assets/icons/circle-help.svg')" aria-hidden="true"></span>
            <div>
                <strong>Serve aiuto?</strong>
                <?php if ($user): ?>
                    <a href="/supporto/">Contatta il supporto</a>
                <?php else: ?>
                    <a href="/?page=login">Accedi per contattarci</a>
                <?php endif; ?>
            </div>
        </div>
    </aside>

    <main class="docs-main">
        <nav class="docs-breadcrumb" aria-label="Percorso">
            <a href="/">Home</a><span aria-hidden="true">/</span><span>Documentazione</span>
        </nav>
        <header class="docs-hero">
            <span class="eyebrow">Manuale di easyPDF</span>
            <h1>Tutto quello che puoi fare con i tuoi PDF</h1>
            <p>Consulta gli strumenti nello stesso ordine dell’editor web e desktop. Ogni colore e ogni icona corrispondono alla funzione che troverai nell’applicazione.</p>
            <div class="docs-stats" aria-label="Contenuto della guida">
                <span><strong><?= count($groups) ?></strong> gruppi di strumenti</span>
                <span><strong><?= $functionCount ?></strong> funzionalità documentate</span>
                <span><strong>Web + desktop</strong> interfaccia uniforme</span>
            </div>
        </header>

        <section class="docs-start" aria-labelledby="iniziare-title">
            <div class="docs-start-icon">
                <span class="tool-icon" style="--tool-icon:url('/assets/icons/book-open.svg')" aria-hidden="true"></span>
            </div>
            <div>
                <span class="docs-kicker">Prima di iniziare</span>
                <h2 id="iniziare-title">Come usare questa guida</h2>
                <p>Apri <strong>Tutti gli strumenti</strong>, scegli il gruppo e poi la funzione desiderata. Puoi trascinare strumenti e voci per riordinarli; con il tasto destro li aggiungi o rimuovi dagli <strong>Strumenti consigliati</strong>. Le preferenze restano memorizzate e, quando disponibile, vengono sincronizzate con l’account.</p>
            </div>
        </section>

        <div id="docs-empty" class="docs-empty" hidden>
            <span class="tool-icon" style="--tool-icon:url('/assets/icons/file-search.svg')" aria-hidden="true"></span>
            <h2>Nessun risultato</h2>
            <p>Prova con il nome di uno strumento o di una funzione, per esempio “OCR”, “firma” o “cover”.</p>
        </div>

        <article class="docs-content" aria-label="Funzionalità di easyPDF">
            <?php foreach ($groups as $number => $group): ?>
                <section class="docs-group" id="<?= e($group['id']) ?>" data-doc-group style="--tool-color:<?= e($group['color']) ?>">
                    <header class="docs-group-head">
                        <div class="docs-group-icon">
                            <span class="tool-icon" style="--tool-icon:url('/assets/icons/<?= e($group['icon']) ?>.svg');--tool-color:<?= e($group['color']) ?>" aria-hidden="true"></span>
                        </div>
                        <div>
                            <span class="docs-kicker">Strumento <?= str_pad((string) ($number + 1), 2, '0', STR_PAD_LEFT) ?></span>
                            <h2><?= e($group['title']) ?></h2>
                            <p><?= e($group['summary']) ?></p>
                        </div>
                        <a class="docs-heading-link" href="#<?= e($group['id']) ?>" aria-label="Collegamento a <?= e($group['title']) ?>">#</a>
                    </header>
                    <div class="docs-group-body">
                        <?php foreach ($group['sections'] as $section): ?>
                            <section class="docs-section" data-doc-section>
                                <h3><?= e($section['title']) ?></h3>
                                <dl>
                                    <?php foreach ($section['items'] as [$label, $description, $itemIcon]): ?>
                                        <div data-doc-item>
                                            <dt>
                                                <span class="tool-icon docs-item-icon" style="--tool-icon:url('/assets/icons/<?= e($itemIcon) ?>.svg');--tool-color:<?= e($group['color']) ?>" aria-hidden="true"></span>
                                                <?= e($label) ?>
                                            </dt>
                                            <dd><?= e($description) ?></dd>
                                        </div>
                                    <?php endforeach; ?>
                                </dl>
                            </section>
                        <?php endforeach; ?>
                    </div>
                    <a class="back-to-index" href="#top">Torna all’inizio <span aria-hidden="true">↑</span></a>
                </section>
            <?php endforeach; ?>
        </article>
    </main>
</div>
<footer class="docs-footer">easyPDF by Massimo Bertolucci · <a href="/documentazione/">Documentazione</a></footer>
</body>
</html>
