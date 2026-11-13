<?php
declare(strict_types=1);

/**
 * User-facing documentation for every tool currently exposed by the catalogue.
 * Planned catalogue entries stay out of this list until they are usable in the UI.
 *
 * @return list<array{id:string,title:string,summary:string,icon:string,color:string,sections:list<array{title:string,items:list<array{0:string,1:string,2:string}>}>}>
 */
function documentation_groups(): array
{
    $groups = [
        ['id' => 'export', 'title' => 'Esporta un PDF', 'summary' => 'Converte il documento aperto in un formato riutilizzabile.', 'sections' => [[
            'title' => 'Esporta in', 'items' => [
                ['Microsoft Word (.docx)', 'Ricostruisce testo, immagini e struttura delle pagine in un documento Word modificabile. Controlla sempre impaginazione e font dopo l’esportazione.'],
                ['Presentazione (.pptx)', 'Crea una presentazione PowerPoint mantenendo ogni pagina PDF come contenuto della diapositiva corrispondente.'],
                ['Immagine (PNG)', 'Renderizza le pagine selezionate come immagini PNG, adatte a pubblicazione e anteprima.'],
                ['Esporta tutte le immagini', 'Estrae in una sola operazione le immagini incorporate nel PDF senza doverle selezionare una per volta.'],
                ['Pagina web HTML', 'Esporta il contenuto leggibile del documento in una pagina HTML consultabile con un browser.'],
                ['Rich Text Format (.rtf)', 'Salva testo e formattazione di base in un file RTF compatibile con molti elaboratori di testo.'],
                ['Testo semplice', 'Estrae il contenuto testuale senza impaginazione in un file di testo UTF-8.'],
            ],
        ]]],
        ['id' => 'fill-sign', 'title' => 'Compila e firma', 'summary' => 'Aggiunge compilazioni e firme visive senza modificare il testo originale.', 'sections' => [[
            'title' => 'Compila', 'items' => [
                ['Aggiungi testo', 'Inserisce una risposta testuale libera nel punto scelto della pagina.'],
                ['Segno di spunta', 'Posiziona un segno di conferma, utile su moduli non interattivi.'],
                ['Croce', 'Inserisce una croce come risposta o marcatura visiva.'],
                ['Punto', 'Inserisce un punto pieno, per esempio su scelte circolari stampate.'],
                ['Linea', 'Aggiunge una breve linea orizzontale ridimensionabile.'],
                ['Data', 'Inserisce la data corrente come elemento compilato modificabile.'],
                ['Firma', 'Posiziona la firma salvata sulla pagina; la firma grafica non equivale automaticamente a una firma digitale con certificato.'],
                ['Iniziali', 'Posiziona le iniziali salvate nel documento.'],
                ['Cambia firma', 'Sostituisce la firma grafica memorizzata per i successivi inserimenti.'],
                ['Cambia iniziali', 'Sostituisce le iniziali memorizzate per i successivi inserimenti.'],
            ],
        ]]],
        ['id' => 'edit', 'title' => 'Modifica un PDF', 'summary' => 'Modifica pagine e contenuti del documento corrente.', 'sections' => [
            ['title' => 'Modifica pagina', 'items' => [
                ['Modifica testo e immagini', 'Seleziona oggetti esistenti per correggere testo, spostare elementi o intervenire sulle immagini compatibili.'],
                ['Ruota pagine', 'Ruota in modo permanente le pagine selezionate di 90, 180 o 270 gradi.'],
                ['Inserisci pagine', 'Aggiunge pagine provenienti da un altro PDF nella posizione scelta.'],
                ['Elimina pagine', 'Rimuove le pagine selezionate dal documento con possibilità di annullare prima del salvataggio.'],
                ['Estrai pagine', 'Crea un nuovo PDF dalle pagine selezionate, lasciando scegliere se rimuoverle dall’originale.'],
                ['Organizza pagine', 'Apre la griglia di miniature per riordinare e gestire più pagine.'],
                ['Applica cover', 'Aggiunge una copertina frontale e, se presente, una copertina posteriore usando file scelti o un kit salvato.'],
            ]],
            ['title' => 'Aggiungi contenuto', 'items' => [
                ['Testo', 'Crea un nuovo blocco di testo con posizione, dimensione e stile configurabili.'],
                ['Immagine', 'Inserisce un’immagine nella pagina e permette di spostarla e ridimensionarla.'],
                ['Intestazione e piè di pagina', 'Aggiunge, aggiorna o rimuove testi ripetuti e numerazioni nelle aree superiore e inferiore.'],
                ['Filigrana', 'Aggiunge, aggiorna o rimuove una scritta o immagine sovrapposta alle pagine.'],
                ['Collegamento', 'Crea un’area cliccabile diretta a una pagina, a un sito web o a un indirizzo email.'],
                ['Numerazione Bates', 'Applica una numerazione progressiva utile per fascicoli legali e documentali.'],
                ['Sfondo', 'Aggiunge, aggiorna o rimuove un colore o contenuto di sfondo sulle pagine selezionate.'],
            ]],
        ]],
        ['id' => 'create', 'title' => 'Crea un PDF', 'summary' => 'Genera nuovi documenti PDF da sorgenti differenti.', 'sections' => [[
            'title' => 'Crea da', 'items' => [
                ['File singolo', 'Converte un file supportato in un nuovo documento PDF.'],
                ['Più file', 'Converte e unisce più file nell’ordine scelto in un singolo PDF.'],
                ['PDF separati', 'Converte più file mantenendo un PDF distinto per ogni sorgente.'],
                ['Immagini', 'Crea pagine PDF a partire da una o più immagini.'],
                ['Appunti', 'Crea un PDF usando un’immagine disponibile negli appunti di sistema.'],
                ['Pagina vuota', 'Apre un nuovo PDF con una pagina bianca nelle dimensioni scelte.'],
            ],
        ]]],
        ['id' => 'combine', 'title' => 'Combina file', 'summary' => 'Raccoglie documenti differenti in un unico PDF.', 'sections' => [[
            'title' => 'Combina', 'items' => [['Aggiungi file da combinare', 'Seleziona più PDF o file supportati, permette di ordinarli e produce un solo documento finale.']],
        ]]],
        ['id' => 'organize', 'title' => 'Organizza pagine', 'summary' => 'Gestisce struttura, ordine e geometria delle pagine tramite miniature.', 'sections' => [
            ['title' => 'Opzioni pagina', 'items' => [
                ['Griglia pagine', 'Mostra tutte le pagine come miniature selezionabili e trascinabili.'],
                ['Ruota', 'Ruota le pagine selezionate.'], ['Elimina', 'Rimuove le pagine selezionate.'],
                ['Estrai', 'Copia le pagine selezionate in un nuovo PDF.'], ['Inserisci', 'Inserisce pagine da un altro documento.'],
                ['Sostituisci', 'Sostituisce le pagine selezionate con pagine provenienti da un altro PDF.'],
                ['Dividi', 'Suddivide il documento secondo numero di pagine, dimensione o intervalli configurati.'],
            ]],
            ['title' => 'Altro', 'items' => [
                ['Imposta riquadri pagina', 'Configura MediaBox, CropBox e gli altri riquadri fisici della pagina.'],
                ['Ritaglia pagine', 'Riduce l’area visibile delle pagine usando un rettangolo di ritaglio.'],
                ['Duplica pagine', 'Crea copie delle pagine selezionate nella posizione desiderata.'],
                ['Inverti pagine', 'Inverte l’ordine dell’intero documento o della selezione.'],
                ['Etichette pagina', 'Imposta etichette logiche, prefissi e numerazioni differenti dal semplice indice pagina.'],
            ]],
        ]],
        ['id' => 'covers', 'title' => 'Applica cover', 'summary' => 'Applica copertine singole o kit personali e condivisi.', 'sections' => [[
            'title' => 'Applica cover', 'items' => [['Applica cover', 'Carica fronte e retro oppure usa il kit predefinito. I kit dell’organizzazione sono sincronizzati dopo l’accesso al relativo server easyPDF.']],
        ]]],
        ['id' => 'comments', 'title' => 'Aggiungi commenti', 'summary' => 'Annota e revisiona il documento senza alterarne direttamente il contenuto.', 'sections' => [
            ['title' => 'Marcatura', 'items' => [
                ['Nota adesiva', 'Aggiunge una nota testuale ancorata a un punto della pagina.'],
                ['Evidenzia testo', 'Applica un’evidenziazione trasparente al testo selezionato.'],
                ['Sottolinea', 'Sottolinea il testo selezionato come annotazione.'],
                ['Barrato', 'Segna il testo selezionato come eliminato o da rivedere.'],
                ['Sottolineatura ondulata', 'Marca il testo con una linea ondulata, tipicamente per segnalare errori.'],
                ['Casella di testo', 'Inserisce un commento visibile direttamente dentro una casella sulla pagina.'],
            ]],
            ['title' => 'Disegno', 'items' => [
                ['Disegno a mano libera', 'Disegna tratti liberi con il puntatore o la penna.'],
                ['Rettangolo', 'Disegna un’annotazione rettangolare configurabile.'], ['Ovale', 'Disegna un’annotazione ellittica.'],
                ['Linea', 'Disegna una linea semplice.'], ['Freccia', 'Disegna una linea con punta direzionale.'],
            ]],
            ['title' => 'Revisione', 'items' => [
                ['Elenco commenti', 'Apre l’elenco filtrabile delle annotazioni presenti nel documento.'],
                ['Appiattisci commenti', 'Incorpora l’aspetto delle annotazioni nel contenuto della pagina rendendole non più modificabili come commenti.'],
            ]],
        ]],
        ['id' => 'ocr', 'title' => 'Scansione e OCR', 'summary' => 'Riconosce testo in scansioni e immagini.', 'sections' => [[
            'title' => 'Riconosci testo', 'items' => [
                ['In questo file', 'Analizza le pagine del PDF aperto e aggiunge un livello testuale ricercabile.'],
                ['In più file', 'Esegue il riconoscimento su una coda di documenti selezionati.'],
                ['Migliora immagine da fotocamera', 'Corregge una fotografia di documento per renderla più leggibile prima dell’OCR.'],
            ],
        ]]],
        ['id' => 'protect', 'title' => 'Proteggi un PDF', 'summary' => 'Controlla accesso, metadati sensibili e sicurezza del documento.', 'sections' => [
            ['title' => 'Proteggi', 'items' => [
                ['Proteggi con password', 'Cifra il PDF impostando password di apertura e autorizzazioni compatibili.'],
                ['Rimuovi informazioni nascoste', 'Analizza e rimuove metadati e contenuti non immediatamente visibili selezionati dall’utente.'],
            ]],
            ['title' => 'Opzioni avanzate', 'items' => [
                ['Proprietà di sicurezza', 'Mostra metodo di cifratura, autorizzazioni e stato di protezione del documento.'],
                ['Rimuovi sicurezza', 'Rimuove la protezione quando l’utente dispone delle credenziali necessarie.'],
            ]],
        ]],
        ['id' => 'redact', 'title' => 'Oscura un PDF', 'summary' => 'Rimuove definitivamente informazioni riservate.', 'sections' => [[
            'title' => 'Oscuramento', 'items' => [
                ['Oscura testo e immagini', 'Marca aree o contenuti e li rimuove in modo permanente quando si applicano gli oscuramenti.'],
                ['Oscura pagine', 'Marca intere pagine per la rimozione visiva del contenuto.'],
                ['Trova testo e oscura', 'Cerca parole o espressioni e crea marcature di oscuramento sui risultati scelti.'],
                ['Imposta proprietà', 'Configura colore e testo sovrapposto usati per le aree oscurate.'],
                ['Bonifica documento', 'Rimuove dati nascosti aggiuntivi prima della distribuzione del file.'],
            ],
        ]]],
        ['id' => 'compress', 'title' => 'Comprimi un PDF', 'summary' => 'Riduce il peso del file con livelli diversi di controllo.', 'sections' => [[
            'title' => 'Ottimizza', 'items' => [
                ['Riduci dimensione file', 'Applica una configurazione rapida di compressione e apre la scelta della destinazione.'],
                ['Ottimizzazione avanzata', 'Permette di regolare immagini, font, oggetti e pulizia prima di salvare il risultato.'],
            ],
        ]]],
        ['id' => 'forms', 'title' => 'Prepara un modulo', 'summary' => 'Crea, configura, compila e distribuisce campi modulo interattivi.', 'sections' => [
            ['title' => 'Campi', 'items' => [
                ['Elenco campi', 'Mostra tutti i campi con nome, tipo e pagina.'], ['Rileva campi modulo', 'Propone automaticamente campi nelle aree riconosciute del documento.'],
                ['Cancella modulo', 'Azzera i valori compilati senza eliminare i campi.'], ['Proprietà campo', 'Modifica nome, aspetto, comportamento, formato e vincoli del campo selezionato.'],
                ['Ordine tabulazione per righe', 'Ordina la navigazione da tastiera seguendo le righe.'], ['Ordine tabulazione per colonne', 'Ordina la navigazione da tastiera seguendo le colonne.'],
                ['Appiattisci campi modulo', 'Trasforma valori e aspetti dei campi in contenuto statico.'], ['Importa dati', 'Compila i campi usando un file dati supportato.'],
                ['Esporta dati', 'Salva nomi e valori dei campi in un formato dati riutilizzabile.'],
            ]],
            ['title' => 'Aggiungi componenti modulo', 'items' => [
                ['Campo di testo', 'Inserisce un campo per testo libero.'], ['Casella di controllo', 'Inserisce una scelta indipendente attivabile.'],
                ['Pulsante di opzione', 'Inserisce una scelta esclusiva appartenente a un gruppo.'], ['Elenco a discesa', 'Inserisce una selezione compatta con valori predefiniti.'],
                ['Casella elenco', 'Mostra più opzioni in un elenco visibile.'], ['Pulsante', 'Inserisce un pulsante configurabile con un’azione.'],
                ['Campo immagine', 'Permette al compilatore di scegliere un’immagine.'], ['Campo data', 'Inserisce un campo predisposto per valori data.'],
                ['Firma digitale', 'Crea un campo destinato a una firma digitale con certificato.'],
            ]],
        ]],
        ['id' => 'stamps', 'title' => 'Aggiungi un timbro', 'summary' => 'Applica timbri grafici alle pagine.', 'sections' => [[
            'title' => 'Timbri', 'items' => [['Tavolozza timbri', 'Apre la raccolta dei timbri disponibili e permette di posizionarli come annotazioni.']],
        ]]],
        ['id' => 'certificates', 'title' => 'Usa un certificato', 'summary' => 'Firma e verifica documenti usando certificati digitali.', 'sections' => [[
            'title' => 'Certificati', 'items' => [
                ['Firma digitalmente', 'Appone una firma crittografica usando identità e certificato selezionati.'],
                ['Convalida tutte le firme', 'Controlla integrità, certificati e stato noto di tutte le firme presenti.'],
                ['Certifica con firma visibile', 'Certifica il documento mostrando anche l’aspetto grafico della firma.'],
                ['Certifica con firma invisibile', 'Certifica il documento senza aggiungere un riquadro grafico alla pagina.'],
            ],
        ]]],
        ['id' => 'print-production', 'title' => 'Produzione di stampa', 'summary' => 'Raccoglie gli strumenti disponibili per la geometria di stampa.', 'sections' => [[
            'title' => 'Produzione di stampa', 'items' => [['Imposta riquadri pagina', 'Configura i riquadri fisici usati per visualizzazione, rifilo, abbondanza e stampa.']],
        ]]],
        ['id' => 'measure', 'title' => 'Misura oggetti', 'summary' => 'Esegue misure tecniche sulle pagine usando una scala configurabile.', 'sections' => [[
            'title' => 'Misura', 'items' => [
                ['Distanza', 'Misura la distanza tra due punti.'], ['Perimetro', 'Somma le lunghezze di una spezzata tracciata.'],
                ['Area', 'Calcola l’area di un poligono tracciato.'], ['Scala disegno', 'Definisce la corrispondenza tra unità della pagina e misura reale.'],
                ['Informazioni misurazione', 'Mostra valori e proprietà delle misurazioni create.'], ['Aggancio', 'Aggancia i punti a estremi, intersezioni e geometrie riconosciute.'],
                ['Esporta misurazioni', 'Salva l’elenco delle misure in un file riutilizzabile.'],
            ],
        ]]],
        ['id' => 'compare', 'title' => 'Confronta file', 'summary' => 'Individua differenze tra due versioni di un documento.', 'sections' => [[
            'title' => 'Confronta', 'items' => [['Seleziona file da confrontare', 'Confronta il PDF corrente con una versione precedente e presenta le differenze rilevate.']],
        ]]],
        ['id' => 'actions', 'title' => 'Azioni guidate', 'summary' => 'Esegue procedure ripetibili composte da più passaggi.', 'sections' => [[
            'title' => 'Elenco azioni', 'items' => [
                ['Procedura guidata azioni', 'Crea, modifica ed esegue sequenze di operazioni su uno o più file.'],
                ['Prepara per la distribuzione', 'Esegue controlli e pulizie utili prima di condividere un documento.'],
                ['Ottimizza documenti acquisiti', 'Esegue una procedura coordinata di OCR e ottimizzazione su scansioni.'],
            ],
        ]]],
        ['id' => 'accessibility', 'title' => 'Prepara per l’accessibilità', 'summary' => 'Analizza e migliora la fruibilità del PDF con tecnologie assistive.', 'sections' => [[
            'title' => 'Accessibilità', 'items' => [
                ['Tagga PDF automaticamente', 'Propone e applica una struttura logica quando il documento è compatibile.'],
                ['Tagga campi modulo', 'Collega i campi modulo alla struttura accessibile del documento.'],
                ['Modifica opzioni di lettura', 'Configura ordine e comportamento usati dai lettori assistivi.'],
                ['Controlla accessibilità', 'Esegue i controlli disponibili e produce un elenco di problemi e verifiche manuali.'],
                ['Trova contenuto senza tag', 'Individua elementi di pagina non collegati alla struttura logica.'],
                ['Apri rapporto accessibilità', 'Mostra il rapporto generato dal controllo con stato e dettagli delle regole.'],
                ['Aggiungi testo alternativo', 'Aggiunge o modifica la descrizione testuale di immagini e figure.'],
                ['Pannello tag', 'Mostra e modifica l’albero strutturale dei tag PDF.'],
            ],
        ]]],
        ['id' => 'standards', 'title' => 'Applica standard PDF', 'summary' => 'Converte il documento verso profili PDF standardizzati disponibili.', 'sections' => [[
            'title' => 'Standard', 'items' => [['Salva come PDF/A', 'Converte e controlla il documento per l’archiviazione a lungo termine secondo il livello PDF/A disponibile.']],
        ]]],
        ['id' => 'ai', 'title' => 'Assistente AI', 'summary' => 'Usa il provider configurato dall’utente o condiviso dall’organizzazione; è disattivato finché non viene configurato.', 'sections' => [[
            'title' => 'Usa il tuo modello', 'items' => [
                ['Riepiloga', 'Invia al provider AI configurato il testo estratto per produrre un riepilogo del documento.'],
                ['Fai domande su questo documento', 'Risponde a domande usando il contenuto del PDF corrente come contesto.'],
                ['Traduci', 'Traduce il testo estratto nella lingua scelta usando il modello configurato.'],
            ],
        ]]],
    ];

    // Keep the documentation visually aligned with the canonical tool catalogue. These are
    // the same Lucide icon names and RGB hues used by crates/engine/src/catalog.rs.
    $appearance = [
        'export' => ['file-output', '#3A6FE8'],
        'fill-sign' => ['pen-line', '#8E4EE6'],
        'edit' => ['file-pen-line', '#D63B8F'],
        'create' => ['file-plus-2', '#E03E3E'],
        'combine' => ['files', '#3A6FE8'],
        'organize' => ['layout-grid', '#2D9D5B'],
        'covers' => ['book-open', '#D63B8F'],
        'comments' => ['message-square-text', '#E88A1A'],
        'ocr' => ['scan-text', '#2D9D5B'],
        'protect' => ['shield-check', '#3A6FE8'],
        'redact' => ['rectangle-horizontal', '#E03E3E'],
        'compress' => ['file-down', '#E03E3E'],
        'forms' => ['text-cursor-input', '#8E4EE6'],
        'stamps' => ['stamp', '#8E4EE6'],
        'certificates' => ['badge-check', '#149CA8'],
        'print-production' => ['printer', '#8E4EE6'],
        'measure' => ['ruler', '#D63B8F'],
        'compare' => ['git-compare', '#D63B8F'],
        'actions' => ['list-checks', '#8E4EE6'],
        'accessibility' => ['accessibility', '#8E4EE6'],
        'standards' => ['file-check', '#E03E3E'],
        'ai' => ['sparkles', '#D63B8F'],
    ];

    // Item order mirrors the Ready entries in the canonical Rust catalogue. Keeping this data
    // beside the prose makes a mismatch visible instead of silently showing the wrong icon.
    $itemIcons = [
        'export' => [['file-text', 'presentation', 'image', 'image', 'file-symlink', 'file-text', 'type']],
        'fill-sign' => [['type', 'check', 'x', 'circle-dot', 'minus', 'clock-3', 'signature', 'signature', 'signature', 'signature']],
        'edit' => [
            ['text-cursor-input', 'rotate-cw', 'file-plus-2', 'trash-2', 'file-output', 'layout-grid', 'book-open'],
            ['type', 'image-plus', 'heading', 'stamp', 'link-2', 'hash', 'palette'],
        ],
        'create' => [['file-input', 'files', 'files', 'image', 'copy-plus', 'file-plus-2']],
        'combine' => [['files']],
        'organize' => [
            ['layout-grid', 'rotate-cw', 'trash-2', 'file-output', 'file-plus-2', 'replace', 'scissors'],
            ['square-dashed-mouse-pointer', 'crop', 'copy-plus', 'arrow-up-down', 'tag'],
        ],
        'covers' => [['book-open']],
        'comments' => [
            ['sticky-note', 'highlighter', 'underline', 'strikethrough', 'spline', 'type'],
            ['pencil', 'square', 'circle', 'minus', 'move-right'],
            ['message-square-text', 'layers'],
        ],
        'ocr' => [['scan-text', 'files', 'camera']],
        'protect' => [['lock', 'eye-off'], ['shield-check', 'lock-open']],
        'redact' => [['rectangle-horizontal', 'file-x', 'file-search', 'settings-2', 'sparkles']],
        'compress' => [['file-down', 'settings-2']],
        'forms' => [
            ['list', 'scan', 'eraser', 'settings-2', 'rows-3', 'columns-3', 'layers', 'file-input', 'file-output'],
            ['text-cursor-input', 'check-circle-2', 'circle', 'chevron-down', 'list', 'square', 'image', 'clock-3', 'signature'],
        ],
        'stamps' => [['stamp']],
        'certificates' => [['signature', 'badge-check', 'badge-check', 'badge-check']],
        'print-production' => [['square-dashed-mouse-pointer']],
        'measure' => [['ruler', 'ruler', 'ruler', 'ruler', 'ruler', 'ruler', 'file-output']],
        'compare' => [['git-compare']],
        'actions' => [['list-checks', 'send', 'scan-text']],
        'accessibility' => [['tag', 'tag', 'book-open', 'accessibility', 'search', 'file-text', 'image', 'tag']],
        'standards' => [['file-check']],
        'ai' => [['sparkles', 'message-circle-reply', 'languages']],
    ];

    foreach ($groups as &$group) {
        [$group['icon'], $group['color']] = $appearance[$group['id']] ?? ['file-text', '#4EB3EB'];
        foreach ($group['sections'] as $sectionIndex => &$section) {
            foreach ($section['items'] as $itemIndex => &$item) {
                $item[] = $itemIcons[$group['id']][$sectionIndex][$itemIndex] ?? $group['icon'];
            }
            unset($item);
        }
        unset($section);
    }
    unset($group);

    return $groups;
}
