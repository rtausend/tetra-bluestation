# 5-B1.1 Detaillierter Implementierungsplan: Testgeraeteleistung erfassen

## Ziel

Jeden Lauf mit belastbaren Pegelmetadaten versehen (`test_tx_power_dbm`, `test_level_dbm`).

## Scope

- Quellprioritaet CLI > Config.
- Pflichtmetadaten fuer Testmodus.

## Betroffene Dateien

- `bins/bluestation-bs/src/main.rs`
- `crates/tetra-config/src/bluestation/sec_phy_soapy.rs`

## Quellenmodell

Pflichtfelder im Testmodus:

- `test_tx_power_dbm`
- `test_level_dbm`

Aufloesungsstrategie:

1. CLI
2. Config

## Implementierungsschritte

1. CLI Argumente fuer beide Felder einfuehren.
2. Merge-Logik fuer beide Quellen implementieren.
3. Fehlermeldung mit Handlungsanweisung bei fehlenden Werten.
4. Metadaten in Ergebnisobjekt und Export aufnehmen.
5. Garantieren, dass ein gestarteter Testlauf keine Nutzerinteraktion mehr benoetigt.

## Tests

- Unit Test Quellprioritaet.
- Integrationstest mit nur Config.
- Integrationstest mit CLI Override.
- Negativtest ohne Werte: Start wird mit klarer Fehlermeldung verweigert.

## Abnahme

- Jeder Testlauf hat beide Metadaten gesetzt.
- Quellprioritaet verhaelt sich deterministisch.
