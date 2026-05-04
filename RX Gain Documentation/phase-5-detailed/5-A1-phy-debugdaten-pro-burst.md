# 5-A1 Detaillierter Implementierungsplan: PHY Debugdaten pro Burst

## Ziel

PHY Ereignisse so erweitern, dass pro Burst nachvollziehbar ist, warum ein Burst spaeter dekodierbar oder nicht dekodierbar war.

## Scope

- Erweiterung der Burst-Metadaten im PHY Datenfluss.
- Strukturierte Logs in `rx_tpsap_prim()`.
- Keine Verhaltensaenderung im eigentlichen Demodulationspfad.

## Betroffene Dateien

- `crates/tetra-entities/src/phy/components/demodulator.rs`
- `crates/tetra-entities/src/phy/phy_bs.rs`
- optional: gemeinsame Typdefinitionen in PHY Komponenten

## Datenmodell

Ein neues oder erweitertes Metadatenobjekt mit mindestens:

- `train_type`
- `train_errs`
- `burst_pos`
- `burst_len`
- `slot_kind` (fullslot/subslot)
- `tdma_time` (falls vorhanden)

## Implementierungsschritte

1. In `SlotBurstFinder` Metadaten konsistent fuellen, auch bei Grenzfaellen (knappe Korrelation, subslot/fullslot).
2. Metadaten vom Finder bis in den Empfaengerpfad propagieren (z. B. via `RxBurstBits` Erweiterung).
3. In `phy_bs.rs` strukturierte Logausgabe einbauen (einheitliches Logformat, feste Feldreihenfolge).
4. Logging so kapseln, dass spaeter CSV/JSON Hook moeglich bleibt (z. B. Helper fuer serialisierbares Event).
5. Sicherstellen, dass fehlende Felder nicht panicen (Defaultwerte/Optionen).

## Fehlerbehandlung

- Unvollstaendige Metadaten als `unknown` markieren statt Drop.
- Keine harte Abbruchlogik in A1 einfuehren.

## Tests

- Unit Test fuer Metadatenfuellung in `demodulator.rs`.
- Unit Test fuer Mapping fullslot/subslot.
- Integrationstest mit Replay (`ul_input_file`) und Snapshot auf Logfelder.

## Abnahme

- Pro Burst sind `train_errs`, `burst_pos`, `train_type`, Slottyp sichtbar.
- Keine Regression in bestehender Burstweitergabe an LMAC.

## Risiken

- Logvolumen steigt stark an.
- Gegenmassnahme: Loglevel und Rate-Limit fuer Detailmodus.
