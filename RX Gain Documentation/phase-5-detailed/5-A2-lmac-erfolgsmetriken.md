# 5-A2 Detaillierter Implementierungsplan: LMAC Erfolgsmetriken konsistent loggen

## Ziel

End-to-End Nachvollziehbarkeit vom PHY Ereignis bis zum LMAC Decodergebnis (inklusive CRC) herstellen.

## Scope

- Standardisiertes Messlog fuer `rx_blk_control()` und `rx_blk_traffic()`.
- Korrelationsschluessel zum PHY Ereignis.

## Betroffene Dateien

- `crates/tetra-entities/src/lmac/lmac_bs.rs`
- optional `crates/tetra-entities/src/lmac/components/errorcontrol.rs`

## Event-Schema

Pflichtfelder pro LMAC-Ereignis:

- `tdma_time`
- `slot_kind`
- `logical_channel`
- `block_num`
- `decode_attempted` (bool)
- `decode_success` (bool)
- `crc_ok` (bool)

## Implementierungsschritte

1. Einheitliches Mess-Struct fuer LMAC-Auswertung anlegen.
2. In Control- und Traffic-Pfad identische Feldlogik anwenden.
3. PHY-LMAC Korrelation ueber `tdma_time + slot_kind` verbindlich durchziehen.
4. Fehlende Decodergebnisse explizit als `decode_attempted=false` markieren.
5. Logging ueber zentrale Funktion ausgeben, um Formatdrift zu verhindern.

## Tests

- Unit Test fuer Event-Schema Vollstaendigkeit.
- Integrationstest mit bekannten Bursts: erwartete Anzahl `decode_attempted` vs `crc_ok`.
- Negativtest: Burst erkannt, aber nicht decodiert -> korrektes Flagging.

## Abnahme

- Fuer jeden relevanten Burst ist LMAC-Ausgang eindeutig klassifizierbar.
- Eventformat ist in Control und Traffic identisch.

## Risiken

- Doppellogging in mehreren Ebenen.
- Gegenmassnahme: ein zentrales Logging Gateway fuer Messereignisse.
