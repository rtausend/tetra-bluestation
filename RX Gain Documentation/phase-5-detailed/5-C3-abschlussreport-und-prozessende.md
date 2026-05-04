# 5-C3 Detaillierter Implementierungsplan: Abschlussreport und Prozessende

## Ziel

Nach Sweep-Ende einen klaren Abschlussreport erzeugen und den Testmodus kontrolliert beenden.

## Scope

- Kompakter Human-Report im Terminal/Log.
- Definierte Exit-Semantik.

## Betroffene Dateien

- `bins/bluestation-bs/src/main.rs`
- optional neue Utility-Datei fuer Reportformatierung

## Reportinhalt

Pflichtbestandteile:

- bestes Gain-Set
- Top-3 Ranking
- Testgeraeteleistung
- Signal/Burstmodus
- Sampleanzahl
- zentrale Kennzahlen (CRC, FP, train_errs)
- 4/4 pass/fail je Pegelpunkt
- niedrigster 4/4 stabiler Pegel

## Implementierungsschritte

1. Reporter-Modul entwerfen (Input: aggregierte Stats).
2. Ausgabeformat fixieren (lesbar, stabile Reihenfolge).
3. Exit-Entscheidung an Laufstatus koppeln.
4. Bei Teilerfolg klares Warning-Label ausgeben.
5. Cleanup garantieren, auch bei Fehlern im Reporting.

## Tests

- Unit Test fuer Rankingdarstellung.
- Snapshot-Test auf Reporttext.
- Integrationstest Exit-Code und Cleanup.

## Abnahme

- Abschlussreport ist direkt fuer Betriebsentscheidungen nutzbar.
- Prozessende ist sauber und reproduzierbar.
