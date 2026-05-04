# 5-B0 Detaillierter Implementierungsplan: Eindeutiger RX Teststartmodus

## Ziel

Einen strikt getrennten Testmodus einführen, der reproduzierbar startet, laeuft und beendet.

## Scope

- CLI Schalter `--rx-gain-test`.
- Kontrollierter Exit inklusive Fehlercodes.

## Betroffene Dateien

- `bins/bluestation-bs/src/main.rs`
- `crates/tetra-config/src/bluestation/config.rs`

## CLI und Laufzeitverhalten

- Ohne Flag: unveraenderter Normalbetrieb.
- Mit Flag: Testmodus-Pipeline aktiv.
- Exit-Code 0 bei erfolgreichem Abschluss.
- Exit-Code != 0 bei Konfigurations- oder Laufzeitfehlern.

## Implementierungsschritte

1. CLI Parsing um Testflag erweitern.
2. Modus-Routing in `main.rs` einbauen (normal vs test).
3. Testmodus Lifecycle kapseln (init -> run -> report -> exit).
4. Fehlerklassen definieren (ConfigError, RuntimeError, NoSamplesError).
5. Abschluss und Cleanup robust sicherstellen (Streams, Files, Handles).

## Tests

- Unit Test fuer CLI Parsing.
- Integrationstest normaler Start ohne Flag.
- Integrationstest Testmodus mit gueltiger Config.
- Negativtest fuer invalide Testconfig.

## Abnahme

- Testmodus ist klar vom Normalmodus getrennt.
- Exit-Codes sind deterministisch und dokumentiert.
