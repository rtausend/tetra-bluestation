# Phase 5 Detailed Implementation Plans

Diese Sammlung enthaelt fuer jeden Teilstep aus Phase 5 einen separaten, detaillierten Implementierungsplan.

## Struktur

- [5-A1 PHY Debugdaten pro Burst](5-A1-phy-debugdaten-pro-burst.md)
- [5-A2 LMAC Erfolgsmetriken](5-A2-lmac-erfolgsmetriken.md)
- [5-A3 Optionale RF Vorqualifizierung](5-A3-rf-vorqualifizierung.md)
- [5-B0 RX Teststartmodus](5-B0-rx-teststartmodus.md)
- [5-B1 Rangebasierte Gain Sweep Konfiguration](5-B1-rangebasierte-gain-sweep-konfiguration.md)
- [5-B1.1 Testgeraeteleistung erfassen](5-B1-1-testgeraeteleistung-erfassen.md)
- [5-B2 Laufzeitwechsel RX Gains](5-B2-laufzeitwechsel-rx-gains.md)
- [5-B3 Testgeraete Architektur](5-B3-testgeraete-architektur.md)
- [5-C1 In-Memory Statistik](5-C1-in-memory-statistik.md)
- [5-C2 Export Offline Auswertung](5-C2-export-offline-auswertung.md)
- [5-C3 Abschlussreport und Prozessende](5-C3-abschlussreport-und-prozessende.md)
- [5-D1 Definierter Testablauf](5-D1-definierter-testablauf.md)
- [5-D2 Near-Far Szenario](5-D2-near-far-szenario.md)
- [5-D3 Entscheidungsregeln](5-D3-entscheidungsregeln.md)

## Gemeinsame Leitlinien fuer alle Teilsteps

- Rueckwaertskompatibilitaet zum Normalbetrieb hat Vorrang.
- TETRA End-to-End Kriterien (Detect -> Decode -> CRC) sind primaer.
- T1 4/4 Kriterium und Near-Far Robustheit bleiben verpflichtende Bewertungsachsen.
- Logging und Export sollen reproduzierbar und maschinenlesbar sein.
