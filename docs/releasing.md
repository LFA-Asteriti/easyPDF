# Rilasciare easyPDF

La versione del pacchetto è in `[workspace.package] version` in `Cargo.toml`.
Il sorgente easyPDF viene proposto con una PR verso `main`. Il rilascio parte solo
dopo il merge e dopo i controlli CI. Non si pubblica direttamente da una copia
di staging: prima si verifica che l'overlay produca gli stessi file.

## Pacchetti desktop

`.github/workflows/release.yml` è un workflow **manuale**. Eseguendolo da
`main` costruisce:

| Sistema | File |
| --- | --- |
| Windows x64 | installer `.msi` e archivio portatile `.zip` |
| macOS Apple Silicon e Intel | `.dmg` universale e archivio CLI `.zip` |
| Linux x86_64 | archivio applicazione `.tar.gz` e archivio CLI `.tar.gz` |

Il workflow produce anche `easypdf-web-host-<versione>.zip` per il portale
autenticato. Include le traduzioni da `craft-i18n` e i font autorizzati da
`craft-fonts`, entrambi bloccati a commit specifici. Gli script di packaging
scaricano i modelli OCR descritti in `ATTRIBUTION.toml` e ne controllano gli hash.
Le tre compilazioni devono riuscire prima che il workflow crei una **bozza**
GitHub con i pacchetti e `SHA256SUMS.txt`. Controllare ogni download e pubblicare
la bozza solo dopo la prova d'installazione e avvio.

I pacchetti attuali non sono firmati. Windows può mostrare SmartScreen. La firma
macOS è ad hoc e non è notarizzata: Gatekeeper può richiedere un'apertura
manuale nelle Impostazioni di sistema. Il workflow non usa credenziali di firma
e non aggiorna automaticamente le installazioni esistenti.

## Portale web

La versione web richiede il portale PHP autenticato, il suo database, lo
storage privato e `easypdf-cli` sul server. Non esiste uno ZIP statico
dell'editor da pubblicare da solo: `packaging/web/package.sh` lo rifiuta.
Provare il pacchetto web dello stesso rilascio su staging, verificare PHP,
CLI, database, storage e permessi sul server, quindi
distribuirlo nel docroot del dominio. Tenere `.env`, codice non pubblico e
documenti condivisi fuori da `public_html`. Fare un backup verificabile prima
del cambio e mantenere una procedura di rollback.

Il sito `easypdf.asteritilfa.it` e il suo contenuto vengono distribuiti
separatamente dai pacchetti desktop. Non usare `compose.yaml` in produzione:
contiene credenziali di test.
