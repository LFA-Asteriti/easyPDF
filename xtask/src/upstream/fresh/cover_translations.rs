//! Keep easyPDF's own cover-tool label translated across the shared web/desktop UI.
//! Existing translations come from easyPDF's earlier local catalogs; the remaining
//! locale labels are original easyPDF wording, pending native-speaker review.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use super::{atomic_write, read_bounded, sha256};

const BUILD_SCRIPT_SHA256: &str = "C5B27F7A90F3E3A297C94881A88ACACCEA50E04C03E7D6C8C3ED1836EFDC5139";
// The independently prepared replay staging differs only in rustfmt layout.
const REPLAY_BUILD_SCRIPT_SHA256: &str = "7496510E5C501E75C1CEB71124B3493109D87E20B76C4F1B745C361A10A0C800";
const COVER_TRANSLATIONS: [(&str, &str); 44] = [
    ("ar", "تطبيق الأغلفة"),
    ("bg", "Прилагане на корици"),
    ("bn", "প্রচ্ছদ প্রয়োগ করুন"),
    ("bs", "Primijeni naslovnice"),
    ("ca", "Aplica les portades"),
    ("ckb", "بەرگەکان جێبەجێ بکە"),
    ("cs", "Použít obálky"),
    ("da", "Anvend forsider"),
    ("de", "Umschläge anwenden"),
    ("dv", "ކަވަރ ޞަފްޙާތައް ހިންގާ"),
    ("el", "Εφαρμογή εξωφύλλων"),
    ("es", "Aplicar portadas"),
    ("fa", "اعمال جلدها"),
    ("fi", "Lisää kannet"),
    ("fr", "Appliquer les couvertures"),
    ("he", "החלת עמודי שער"),
    ("hi", "कवर लागू करें"),
    ("hu", "Borítók alkalmazása"),
    ("id", "Terapkan sampul"),
    ("it", "Applica cover"),
    ("ja", "表紙を適用"),
    ("ka", "თავფურცლების გამოყენება"),
    ("kk", "Мұқабаларды қолдану"),
    ("ko", "표지 적용"),
    ("lo", "ນຳໃຊ້ໜ້າປົກ"),
    ("lt", "Taikyti viršelius"),
    ("nb", "Bruk omslag"),
    ("nl", "Voorbladen toepassen"),
    ("pl", "Zastosuj okładki"),
    ("pt-br", "Aplicar capas"),
    ("ro", "Aplică coperțile"),
    ("ru", "Применить обложки"),
    ("sl", "Uporabi naslovnice"),
    ("sq", "Apliko kopertinat"),
    ("sv", "Använd omslagssidor"),
    ("ta", "அட்டைப் பக்கங்களைப் பயன்படுத்து"),
    ("te", "కవర్‌లను వర్తింపజేయి"),
    ("th", "ใช้หน้าปก"),
    ("tr", "Kapakları uygula"),
    ("uk", "Застосувати обкладинки"),
    ("ur", "سرورق لاگو کریں"),
    ("vi", "Áp dụng bìa"),
    ("zh-hans", "应用封面"),
    ("zh-hant", "套用封面"),
];

const IT_COVER_DIALOG_SOURCE: &str = r#"// easyPDF-specific text absent from the pinned upstream catalogs.
const IT_EASYPDF_TRANSLATIONS: &[(&str, &str)] = &[
    ("Insert every front-cover page before the document and every back-cover page after it.", "Inserisci le pagine della cover frontale prima del documento e quelle della cover posteriore dopo."),
    ("Choose a cover kit", "Scegli un kit cover"),
    ("Front cover", "Cover frontale"),
    ("Back cover", "Cover posteriore"),
    ("Not selected", "Non selezionata"),
    ("Choose PDF…", "Scegli PDF…"),
    ("Save these covers as a kit", "Salva queste cover come kit"),
    ("Cover kit name", "Nome del kit cover"),
    ("Use as default cover kit", "Usa come kit cover predefinito"),
    ("The covers are applied as one undoable change.", "Le cover vengono applicate con un'unica modifica annullabile."),
    ("easyPDF support", "Supporto easyPDF"),
    ("easyPDF documentation", "Documentazione easyPDF"),
    ("A protected snapshot including current unsaved changes will be shared.", "Verrà condivisa una copia protetta che include le modifiche correnti non ancora salvate."),
    ("Administrator", "Amministratore"),
    ("AI configuration to use", "Configurazione AI da usare"),
    ("AI provider", "Provider AI"),
    ("API key", "Chiave API"),
    ("Apply covers", "Applica cover"),
    ("Approve", "Approva"),
    ("Back preview", "Anteprima retro"),
    ("Ban", "Banna"),
    ("Banned", "Bannato"),
    ("Batch outputs: {saved} saved, {downloads} downloads started, {cancelled} cancelled, {failed} failed", "Risultati del gruppo: {saved} salvati, {downloads} download avviati, {cancelled} annullati, {failed} non riusciti"),
    ("Browser opens. Login. Allow access. Return here.", "Browser apre. Accedi. Consenti accesso. Torna qui."),
    ("Check for updates when easyPDF starts", "Verifica aggiornamenti all'avvio di easyPDF"),
    ("Choose model", "Scegli modello"),
    ("Configure an AI provider in Preferences first.", "Configura prima un provider AI nelle Preferenze."),
    ("Connect", "Collega"),
    ("Connect easyPDF to your Web installation first.", "Collega prima easyPDF alla tua installazione web."),
    ("Connect the desktop app to your easyPDF Web installation.", "Collega l’app desktop alla tua installazione web di easyPDF."),
    ("Connect to this site", "Collega a questo sito"),
    ("Connect your account to synchronize cover kits.", "Collega l’account per sincronizzare i kit cover."),
    ("Continue with ChatGPT", "Continua con ChatGPT"),
    ("Cover kits are shared with every account in the organization.", "I kit cover sono condivisi con tutti gli account dell’organizzazione."),
    ("Create a concise summary of the open document.", "Crea un riepilogo conciso del documento aperto."),
    ("Create cover kit", "Crea kit cover"),
    ("Create organization", "Crea organizzazione"),
    ("Create user", "Crea utente"),
    ("Current account", "Account corrente"),
    ("Delete organization", "Elimina organizzazione"),
    ("Delete organization and all its users", "Elimina l'organizzazione e tutti i suoi utenti"),
    ("Delete selected", "Elimina selezionati"),
    ("Delete selected users?", "Eliminare gli utenti selezionati?"),
    ("Detach users and make them personal accounts", "Scollega gli utenti e rendili account personali"),
    ("Disable", "Disattiva"),
    ("Download started: {name}", "Download avviato: {name}"),
    ("Download started: {name}; the document is still unsaved", "Download avviato: {name}; il documento non è ancora salvato"),
    ("Dock navigation rail on Left", "Posiziona la barra di navigazione a sinistra"),
    ("Dock navigation rail on Right", "Posiziona la barra di navigazione a destra"),
    ("Edit organization", "Modifica organizzazione"),
    ("Edit user", "Modifica utente"),
    ("Email", "Email"),
    ("Enter /v1 URL. Paste key if required. Load models. Choose model.", "Inserisci URL /v1. Incolla chiave se richiesta. Recupera modelli. Scegli modello."),
    ("Enter the address of the easyPDF Web installation used by your company.", "Inserisci l'indirizzo dell'installazione web easyPDF usata dalla tua azienda."),
    ("Front preview", "Anteprima fronte"),
    ("General & Interface", "Generale e interfaccia"),
    ("Google AI Studio. Create key. Paste key. Load models. Choose model.", "Google AI Studio. Crea chiave. Incolla chiave. Recupera modelli. Scegli modello."),
    ("Initial password", "Password iniziale"),
    ("Invert middle-button drag", "Inverti trascinamento con il pulsante centrale"),
    ("Leave both password fields blank to keep the current password.", "Lascia entrambi i campi password vuoti per mantenere la password attuale."),
    ("Load models", "Recupera modelli"),
    ("Log out", "Esci"),
    ("Mark all as read", "Segna tutte come lette"),
    ("Mark as read", "Segna come letta"),
    ("Navigation rail side", "Lato della barra di navigazione"),
    ("New password", "Nuova password"),
    ("No cover kits saved yet.", "Non ci sono ancora kit cover salvati."),
    ("No documents have been shared with you.", "Nessun documento è stato condiviso con te."),
    ("No notifications.", "Nessuna notifica."),
    ("No organization AI has been shared with this account.", "Nessuna AI dell'organizzazione è stata condivisa con questo account."),
    ("No other active users are available in this organization.", "Non sono disponibili altri utenti attivi in questa organizzazione."),
    ("Notification", "Notifica"),
    ("Notifications", "Notifiche"),
    ("Only organization administrators can manage cover kits.", "Solo gli amministratori dell'organizzazione possono gestire i kit cover."),
    ("OpenAI Platform. Create API key. Paste key. Load models. Choose model.", "OpenAI Platform. Crea chiave API. Incolla chiave. Recupera modelli. Scegli modello."),
    ("OpenRouter. Create key. Paste key. Load models. Choose model.", "OpenRouter. Crea chiave. Incolla chiave. Recupera modelli. Scegli modello."),
    ("No files were produced", "Nessun file prodotto"),
    (
        "The selected file does not match this recent document",
        "Il file selezionato non corrisponde a questo documento recente",
    ),
    ("Organization AI", "AI dell'organizzazione"),
    ("Organization data and cover kits will be deleted.", "I dati dell'organizzazione e i kit cover verranno eliminati."),
    ("Organization name", "Nome organizzazione"),
    ("Organizations", "Organizzazioni"),
    ("Panning or auto-scrolling with the middle (wheel) button moves the page the other way.", "Il trascinamento o lo scorrimento automatico con il pulsante centrale (rotellina) sposta la pagina nella direzione opposta."),
    ("Password must contain at least 8 characters", "La password deve contenere almeno 8 caratteri"),
    ("Passwords do not match", "Le password non coincidono"),
    ("Passwords match", "Le password coincidono"),
    ("Passwords match, but the password is too short", "Le password coincidono, ma sono troppo corte"),
    ("Pending approval", "In attesa di approvazione"),
    ("Personal account", "Account personale"),
    ("Personal AI", "AI personale"),
    ("Press Continue. Login to ChatGPT. Allow access. Done.", "Premi Continua. Accedi a ChatGPT. Consenti accesso. Fatto."),
    ("Question", "Domanda"),
    ("Remove from Recommended tools", "Rimuovi dagli Strumenti consigliati"),
    ("Save AI provider", "Salva provider AI"),
    ("Save changes", "Salva modifiche"),
    ("Save cover kit", "Salva kit cover"),
    ("Saved cover kits", "Kit cover salvati"),
    ("Saved a copy; the open document has newer changes.", "Copia salvata; il documento aperto contiene modifiche più recenti."),
    ("Search settings…", "Cerca impostazioni…"),
    ("Selected users may choose this provider or keep their personal AI.", "Gli utenti selezionati possono scegliere questo provider oppure mantenere la propria AI personale."),
    ("Set as default", "Imposta come predefinito"),
    ("Share document", "Condividi documento"),
    ("Share with organization users", "Condividi con gli utenti dell'organizzazione"),
    ("shared", "ha condiviso"),
    ("Shared by", "Condivisa da"),
    ("Shared documents", "Documenti condivisi"),
    ("Super Admin", "Super Admin"),
    ("Synchronize", "Sincronizza"),
    ("Disconnect", "Disconnetti"),
    ("Disconnected", "Disconnesso"),
    ("The credential stays encrypted on the server and is never shown to organization users.", "La credenziale resta cifrata sul server e non viene mai mostrata agli utenti dell'organizzazione."),
    ("This action cannot be undone.", "Questa azione non può essere annullata."),
    ("Translate to", "Traduci in"),
    ("Unban", "Sblocca"),
    ("URL and email stay on this device. Password and access token stay only in memory.", "URL ed email restano sul dispositivo. Password e token di accesso restano solo in memoria."),
    ("Users", "Utenti"),
    ("Waiting for AI provider…", "Attendo il provider AI…"),
    ("Web installation", "Installazione web"),
    ("{font}, {size} pt", "{font}, {size} pt"),
    ("{n} lines", "{n} righe"),
    ("{n} files prepared; choose destinations", "{n} file preparati; scegli le destinazioni"),
    ("; first write error: {error}", "; primo errore di scrittura: {error}"),
    ("1 line", "1 riga"),
    ("A protected snapshot including current unsaved changes will be shared.", "Verrà condivisa una copia protetta che include le modifiche correnti non ancora salvate."),
    ("Account and organization", "Account e organizzazione"),
    ("Add Tools to Hotbar", "Aggiungi strumenti alla barra rapida"),
    ("AI providers", "Provider AI"),
    ("Background removal", "Rimozione dello sfondo"),
    ("Black and white (binarize)", "Bianco e nero (binarizza)"),
    ("bold", "grassetto"),
    ("Built-in", "Integrato"),
    ("Cancel re-read", "Annulla la nuova lettura"),
    ("Cleans up the scanned pictures in place: recognized text, comments and links on the pages stay as they are.", "Migliora le immagini scansionate senza sostituirle: testo riconosciuto, commenti e collegamenti restano invariati."),
    ("Close menu", "Chiudi menu"),
    ("Combine engines", "Combina i motori"),
    ("Cover kits", "Kit cover"),
    ("Custom scale", "Scala personalizzata"),
    ("Despeckle", "Rimuovi puntini"),
    ("Drag hotbar", "Trascina la barra rapida"),
    ("easyPDF's checks found no problems for {level}. This is not a validator result.", "I controlli di easyPDF non hanno rilevato problemi per {level}. Non è un risultato di convalida."),
    ("easyPDF's checks now find no problems for {level}. This is not a validator result. Save to keep the changes.", "I controlli di easyPDF ora non rilevano problemi per {level}. Non è un risultato di convalida. Salva per mantenere le modifiche."),
    ("Edge shadow removal", "Rimozione delle ombre ai bordi"),
    ("Engine", "Motore"),
    ("Enhance", "Migliora"),
    ("Enhance Scanned File", "Migliora file scansionato"),
    ("Enhancing scans… image {d} of {t}", "Miglioramento scansioni… immagine {d} di {t}"),
    ("Enter /v1 URL. Paste key if required. Load models. Choose model.", "Inserisci URL /v1. Incolla chiave se richiesta. Recupera modelli. Scegli modello."),
    ("Enter the address of the easyPDF Web installation used by your company.", "Inserisci l'indirizzo dell'installazione web easyPDF usata dalla tua azienda."),
    ("Evens out the paper so a grey or yellowed background turns white, and darkens faint strokes. 0 is off.", "Uniforma la carta rendendo bianco uno sfondo grigio o ingiallito e scurisce i tratti sbiaditi. 0 disattiva la funzione."),
    ("Every line set in the style chosen for a level becomes a bookmark of that level, under a new first bookmark.", "Ogni riga con lo stile scelto per un livello diventa un segnalibro di quel livello, sotto un nuovo segnalibro principale."),
    ("Extend the docked toolbar along the entire side. Choose a side above to enable.", "Estende la barra agganciata lungo tutto il lato. Scegli prima un lato."),
    ("Float", "Fluttuante"),
    ("Full-side strip", "Barra sull'intero lato"),
    ("Google AI Studio. Create key. Paste key. Load models. Choose model.", "Google AI Studio. Crea chiave. Incolla chiave. Recupera modelli. Scegli modello."),
    ("Hotbar settings", "Impostazioni barra rapida"),
    ("Hotbar settings / add tools", "Impostazioni barra rapida / aggiungi strumenti"),
    ("Hotbar tools", "Strumenti della barra rapida"),
    ("italic", "corsivo"),
    ("Layout & Sizing", "Disposizione e dimensioni"),
    ("Level {n}", "Livello {n}"),
    ("Lightens shadows along the edges of the scan, such as a book's gutter or a copier's dark border", "Schiarisce le ombre ai bordi della scansione, come la piega di un libro o il bordo scuro di una fotocopia"),
    ("New bookmarks from text styles", "Nuovi segnalibri dagli stili di testo"),
    ("New bookmarks from text styles…", "Nuovi segnalibri dagli stili di testo…"),
    ("No scanned pages were cleaned up.", "Nessuna pagina scansionata è stata migliorata."),
    ("OpenAI Platform. Create API key. Paste key. Load models. Choose model.", "OpenAI Platform. Crea chiave API. Incolla chiave. Recupera modelli. Scegli modello."),
    ("Page Properties", "Proprietà della pagina"),
    ("Panning or auto-scrolling with the middle (wheel) button moves the page the other way.", "Il trascinamento o lo scorrimento automatico con il pulsante centrale (rotellina) sposta la pagina nella direzione opposta."),
    ("Paste bookmarks", "Incolla segnalibri"),
    ("Re-reading the page…", "Nuova lettura della pagina…"),
    ("Re-reading the region…", "Nuova lettura dell'area…"),
    ("Remove from hotbar", "Rimuovi dalla barra rapida"),
    ("Remove scan speckle", "Rimuovi i puntini della scansione"),
    ("Remove this leaf tag from reading order and mark its direct content as an artifact", "Rimuovi questo tag foglia dall'ordine di lettura e segna il suo contenuto diretto come artefatto"),
    ("Removes isolated dark specks such as dust and scanner noise. The only filter black and white scans take.", "Rimuove puntini scuri isolati, come polvere e disturbi dello scanner. È l'unico filtro applicabile alle scansioni in bianco e nero."),
    ("Search all tools…", "Cerca tra tutti gli strumenti…"),
    ("Selected users may choose this provider or keep their personal AI.", "Gli utenti selezionati possono scegliere questo provider oppure mantenere la propria AI personale."),
    ("Some features are not available yet.", "Alcune funzioni non sono ancora disponibili."),
    ("Stick to a side:", "Fissa a un lato:"),
    ("Straighten pages scanned sideways or upside down", "Raddrizza le pagine scansionate di lato o capovolte"),
    ("Straighten skewed text lines", "Raddrizza le righe di testo inclinate"),
    ("Tab Order", "Ordine di tabulazione"),
    ("Tesseract", "Tesseract"),
    ("Text recognition isn't installed: its model files are missing. Reinstall easyPDF, or set EASYPDF_MODELS to the folder that holds them.", "Il riconoscimento del testo non è installato: mancano i file dei modelli. Reinstalla easyPDF o imposta EASYPDF_MODELS sulla cartella che li contiene."),
    ("The credential stays encrypted on the server and is never shown to organization users.", "La credenziale resta cifrata sul server e non viene mai mostrata agli utenti dell'organizzazione."),
    ("The re-read was cancelled; the words are unchanged", "La nuova lettura è stata annullata; le parole sono invariate"),
    ("This document has no text to make bookmarks from.", "Questo documento non contiene testo da cui creare segnalibri."),
    ("Toolbar size", "Dimensione barra degli strumenti"),
    ("Unspecified", "Non specificato"),
    ("URL and email stay on this device. Password and access token stay only in memory.", "URL ed email restano sul dispositivo. Password e token di accesso restano solo in memoria."),
    ("Use Column Order", "Usa l'ordine per colonne"),
    ("Use Document Structure", "Usa la struttura del documento"),
    ("Use Row Order", "Usa l'ordine per righe"),
    ("Vertical layout", "Disposizione verticale"),
    ("3D model", "Modello 3D"),
    ("Add 3D model", "Aggiungi modello 3D"),
    ("Crop image", "Ritaglia immagine"),
    ("Couldn't enhance the scanned pages", "Impossibile migliorare le pagine scansionate"),
    ("Enhance scanned file…", "Migliora file scansionato…"),
    ("Enhancing the scanned pages was cancelled", "Miglioramento delle pagine scansionate annullato"),
    ("Filters", "Filtri"),
    ("Flags", "Indicatori"),
    ("Freehand-Highlight", "Evidenziazione a mano libera"),
    ("Hebrew", "Ebraico"),
    ("Import bookmarks", "Importa segnalibri"),
    ("Look", "Aspetto"),
    ("Page properties…", "Proprietà pagina…"),
    ("Page templates", "Modelli di pagina"),
    ("Page {n}", "Pagina {n}"),
    ("Print pages", "Stampa pagine"),
    ("Remove {standard} identification", "Rimuovi l'identificazione {standard}"),
    ("Reset side panels", "Ripristina i pannelli laterali"),
    ("Scanned pages are already being enhanced", "Il miglioramento delle pagine scansionate è già in corso"),
    ("Select all", "Seleziona tutto"),
    ("Show or hide the Tools panel", "Mostra o nascondi il pannello Strumenti"),
    ("Show or hide the side panel", "Mostra o nascondi il pannello laterale"),
    ("The certificate of {} has critical extensions easyPDF does not recognize, so it is not used to vouch for {}.", "Il certificato di {} contiene estensioni critiche che easyPDF non riconosce, quindi non viene usato per attestare {}."),
    ("The signature's signed attributes have no content type, which CMS requires; it was read anyway.", "Gli attributi firmati non contengono il tipo di contenuto richiesto da CMS; sono stati comunque letti."),
    ("The signature's signed attributes have no message digest, so the signature does not cover the document.", "Gli attributi firmati non contengono l'impronta del messaggio, quindi la firma non copre il documento."),
    ("The signature's signed attributes name a content type other than data, so it is not a signature of the document.", "Gli attributi firmati indicano un tipo di contenuto diverso da data, quindi la firma non riguarda il documento."),
    ("The timestamp authority's certificate is not issued for time stamping.", "Il certificato dell'autorità di marcatura temporale non è destinato alla marcatura temporale."),
    ("This signature uses an RSA key shorter than 2048 bits, which is no longer considered strong enough for signatures. It still validates, but it should not be relied on.", "Questa firma usa una chiave RSA inferiore a 2048 bit, non più considerata abbastanza sicura. La verifica riesce, ma la firma non è affidabile."),
    ("Vertices", "Vertici"),
    ("thick", "spesso"),
    ("A link or special file, which isn't followed", "Un collegamento o file speciale, che non viene seguito"),
    ("Couldn't be read: {reason}", "Impossibile leggere: {reason}"),
    ("Larger than 512 MiB", "Superiore a 512 MiB"),
    ("Listing the folder…", "Elenco della cartella…"),
    ("Load to edit", "Carica per modificare"),
    ("Loading {name}…", "Caricamento di {name}…"),
    ("Lowering this forgets the oldest files. At 0 there is no list.", "Riducendo questo valore vengono dimenticati i file meno recenti. Con 0 non viene conservato alcun elenco."),
    ("Matches: {matches}. Files with matches: {files}. PDFs read to the end: {searched} of {found}.", "Corrispondenze: {matches}. File con corrispondenze: {files}. PDF letti fino alla fine: {searched} di {found}."),
    ("Not searched: {n}", "Non cercati: {n}"),
    ("Output intent: {list}", "Intento di output: {list}"),
    ("Pages that couldn't be read: {n}", "Pagine non leggibili: {n}"),
    ("Reading file {n} of {total}: {name}", "Lettura del file {n} di {total}: {name}"),
    ("Recent files to keep", "File recenti da conservare"),
    ("Search a folder", "Cerca in una cartella"),
    ("Search a folder…", "Cerca in una cartella…"),
    ("Searching a folder needs the desktop app", "La ricerca in una cartella richiede l'app desktop"),
    ("Show the current page number over the document", "Mostra il numero della pagina corrente sopra il documento"),
    ("Stopped at {limit} matches.", "Interrotto al raggiungimento di {limit} corrispondenze."),
    ("Stopped before the end.", "Interrotto prima della fine."),
    ("Stopped: too many files took too long to read.", "Interrotto: troppi file hanno richiesto troppo tempo per essere letti."),
    ("The rest are left out of this list.", "Gli altri elementi non sono inclusi in questo elenco."),
    ("The search stopped unexpectedly.", "La ricerca si è interrotta in modo imprevisto."),
    ("This folder is too large or too deep to search all of it.", "Questa cartella è troppo grande o contiene troppi livelli per essere cercata interamente."),
    ("This large file is being read from disk to save memory, so it can't be edited, filled in or signed yet.", "Questo file di grandi dimensioni viene letto dal disco per risparmiare memoria, quindi non può ancora essere modificato, compilato o firmato."),
    ("To print this large file, load all of it first with Load to edit.", "Per stampare questo file di grandi dimensioni, caricalo prima interamente con Carica per modificare."),
    ("Took too long to read", "Lettura troppo lunga"),
];"#;

pub(super) fn apply(root: &Path, stage: &Path, backup_label: &str) -> Result<()> {
    let relative = "crates/ui-egui/build.rs";
    let original = read_bounded(&stage.join(relative))?;
    let original_hash = sha256(&original);
    if !original_hash.eq_ignore_ascii_case(BUILD_SCRIPT_SHA256) && !original_hash.eq_ignore_ascii_case(REPLAY_BUILD_SCRIPT_SHA256) {
        bail!("review required: staged UI catalog builder changed");
    }
    let backup = root.join("upstream/backups").join(format!("{backup_label}-fresh-cover-translations-before-apply"));
    if backup.exists() {
        bail!("cover translation backup already exists: {}", backup.display());
    }
    let source = String::from_utf8(original.clone()).context("decode UI catalog builder")?;
    let table_anchor = "const APP_CATALOG: &str = \"pdfcraft.tsv\";";
    if source.matches(table_anchor).count() != 1 {
        bail!("review required: UI catalog builder table anchor changed");
    }
    let mut rows = String::new();
    for (locale, translation) in COVER_TRANSLATIONS {
        rows.push_str(&format!("    ({locale:?}, {translation:?}),\n"));
    }
    let table = format!(
        "{table_anchor}\n\n// easyPDF-owned tool label, shared by the web and desktop catalogs.\nconst EASY_PDF_COVER_TRANSLATIONS: &[(&str, &str)] = &[\n{rows}];\n\n{IT_COVER_DIALOG_SOURCE}"
    );
    let mut updated = source.replacen(table_anchor, &table, 1);
    let old = "let branded = source.replace(\"PdfCraft\", \"easyPDF\").replace(\"pdfcraft\", \"easypdf\").replace(\"ArtCraft\", \"easyPDF\");";
    if updated.matches(old).count() != 1 {
        bail!("review required: UI catalog builder branding anchor changed");
    }
    let new = r#"let mut branded = source.replace("PdfCraft", "easyPDF").replace("pdfcraft", "easypdf").replace("ArtCraft", "easyPDF");
        if let Some((_, translation)) = EASY_PDF_COVER_TRANSLATIONS.iter().find(|(locale, _)| *locale == code.as_str()) {
            if branded.contains("\tApply covers\t") || branded.contains("\tApply covers…\t") {
                return Err(format!("{code}: upstream now owns Apply covers translations; review before merging"));
            }
            branded.push_str(&format!("\n\tApply covers\t{translation}\n\tApply covers…\t{translation}…\n"));
        }
        if code == "it" {
            branded.push_str("\n\teasyPDF processes local files on your device. The protected web edition uses your authorized account.\teasyPDF elabora i file locali sul dispositivo. La versione web protetta utilizza il tuo account autorizzato.\n");
            for (source, translation) in IT_EASYPDF_TRANSLATIONS {
                if !branded.lines().any(|line| line.split('\t').nth(1) == Some(*source)) {
                    branded.push_str(&format!("\t{source}\t{translation}\n"));
                }
            }
        }"#;
    updated = updated.replacen(old, new, 1);
    // The replay checkout retained an older comment spelling; keep both outputs byte-identical.
    updated = updated.replace(
        "translated labels and English lookup keys match easyPDF's UI source.",
        "translated labels and their English lookup keys match easyPDF's UI source.",
    );
    let mut formatter = Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("start rustfmt for the UI catalog builder")?;
    {
        let mut stdin = formatter.stdin.take().context("open rustfmt input")?;
        stdin.write_all(updated.as_bytes()).context("send UI catalog builder to rustfmt")?;
    }
    let formatted = formatter.wait_with_output().context("format UI catalog builder")?;
    if !formatted.status.success() {
        bail!("rustfmt rejected UI catalog builder: {}", String::from_utf8_lossy(&formatted.stderr));
    }
    atomic_write(&backup.join(relative), &original)?;
    atomic_write(&stage.join(relative), &formatted.stdout)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::COVER_TRANSLATIONS;

    #[test]
    fn every_embedded_non_english_catalog_has_one_cover_label() {
        let codes: BTreeSet<&str> = COVER_TRANSLATIONS.iter().map(|(code, _)| *code).collect();
        assert_eq!(codes.len(), 44);
        assert!(COVER_TRANSLATIONS.iter().all(|(_, label)| !label.trim().is_empty() && !label.ends_with('…')));
    }
}
