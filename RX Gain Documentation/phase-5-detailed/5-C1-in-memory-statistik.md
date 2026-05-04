# 5-C1 Detaillierter Implementierungsplan: In-Memory Statistik

## Ziel

Eine robuste Laufzeitaggregation aufbauen, die pro Gain/Level/Burst-Typ belastbare Kennzahlen liefert.

## Scope

- Countermodell fuer expected/detected/decoded/crc/false_positive.
- Slotweise Zusatzzaehler fuer 4/4 Kriterium.

## Betroffene Dateien

- `crates/tetra-entities/src/phy/phy_bs.rs`
- optional neue Komponente unter `crates/tetra-entities/src/phy/components/`

## Datenstruktur

Key:

- `gain_combo`
- `test_level_dbm`
- `burst_type`
- `train_type`

Values:

- `expected_slots`
- `detected`
- `decoded`
- `crc_ok`
- `false_positive`
- `slot_crc_ok[4]`
- `slot_expected[4]`

## Implementierungsschritte

1. Stats-Struct mit atomarer oder synchronisierter Aktualisierung einbauen.
2. Updatepunkte im Datenfluss definieren (expected, detected, decoded, crc).
3. Slotindex aus TDMA-Kontext extrahieren und slotweise Counter pflegen.
4. Rolling Window plus kumulative Sicht implementieren.
5. Abgeleitete Kennzahlen als Funktionen bereitstellen (Rates, 4/4 pass/fail).

## Tests

- Unit Tests fuer Counter-Updates.
- Unit Tests fuer Rate-Berechnung inkl. Divide-by-zero Schutz.
- Integrationstest mit Replay und erwarteten Zaehlern.

## Abnahme

- Kennzahlen sind intern konsistent und reproduzierbar.
- 4/4 Auswertung laesst sich aus den Daten eindeutig ableiten.
