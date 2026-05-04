# 5-B2 Detaillierter Implementierungsplan: Laufzeitwechsel der RX Gains

## Ziel

Gain-Kombinationen ohne Prozessneustart durchfahren und pro Kombination robust messen.

## Scope

- Interne Runtime-Schnittstelle fuer Gainwechsel (kein externes User-API).
- Sweep-State-Machine.
- Bestprofilselektion.

## Betroffene Dateien

- `crates/tetra-entities/src/phy/components/soapyio.rs`
- `crates/tetra-entities/src/phy/components/soapy_dev.rs`

## State Machine

- `ApplyGains`
- `Settling`
- `MeasureWindow`
- `FinalizeAndNext`

## Implementierungsschritte

1. API `apply_rx_gain_combo(combo)` in Soapy-Layer einfuehren.
2. Safe switching nur an Slotgrenzen erlauben.
3. Settling-Discard von Slots sauber implementieren.
4. Messfensterlogik auf `window_bursts` stabilisieren.
5. Rankinglogik einbauen:
   - primaer `crc_pass_rate`
   - bei T1 nur 4/4 stabile Kandidaten
   - tie-break ueber `false_positive_rate`, dann `train_errs`
6. Optionalen Plausibilitaetsfilter ueber `rf_probe_db` anbinden.
7. Sicherstellen, dass der Ablauf nach Teststart ohne weitere Eingaben vollautomatisch bis Report/Exit laeuft.

## Parallelitaet und Synchronisation

- Shared State fuer aktuellen Sweepzustand thread-safe halten.
- Keine konkurrierenden Gainwechsel waehrend aktiver Fenstermessung.

## Tests

- Unit Test State-Machine Uebergaenge.
- Integrationstest mehrerer Gainkombinationen.
- Negativtest mit nicht unterstuetztem Gainnamen.

## Abnahme

- Vollstaendiger Sweep laeuft ohne Neustart.
- Bestprofil ist reproduzierbar bei identischem Replay-Input.
