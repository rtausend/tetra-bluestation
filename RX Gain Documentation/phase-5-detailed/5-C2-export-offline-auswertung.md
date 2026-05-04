# 5-C2 Detaillierter Implementierungsplan: Export fuer Offline-Auswertung

## Ziel

Messdaten periodisch und am Laufende in stabilen Formaten exportieren, damit externe Analyse und Regression moeglich sind.

## Scope

- CSV und/oder JSON Export.
- Rankingexport am Ende.

## Betroffene Dateien

- neu: `crates/tetra-entities/src/phy/components/rx_gain_stats.rs`

## Exportschema

Pflichtfelder je Datensatz:

- `timestamp`
- `gain_combo`
- `test_level_dbm`
- `detected`
- `crc_ok_rate`
- `slot0_crc_rate` bis `slot3_crc_rate`
- `train_err_mean`
- `fp_rate`
- `rf_probe_db`
- `test_device_type`
- `test_signal_mode`
- `test_tx_power_dbm`

## Implementierungsschritte

1. Serialisierbare Exportmodelle erstellen.
2. Dateibenennung mit Run-ID und Zeitstempel festlegen.
3. Periodisches Flush-Verhalten definieren (z. B. pro Fenster).
4. Abschlussranking als separates Objekt/Datei schreiben.
5. Fehlerfallstrategie: Exportfehler loggen, Testlauf optional fortsetzen/abbrechen per Config.

## Tests

- Unit Test Serialisierung.
- Integrationstest Dateiinhalt und Spaltenreihenfolge.
- Robustheitstest bei Schreibfehlern.

## Abnahme

- Exportdateien sind maschinenlesbar und feldvollstaendig.
- Rankingdatei entspricht internem Bestprofil.
