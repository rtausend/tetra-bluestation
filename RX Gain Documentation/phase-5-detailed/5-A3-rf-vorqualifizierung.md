# 5-A3 Detaillierter Implementierungsplan: Optionale RF Vorqualifizierung

## Ziel

Vor jedem Messfenster eine schnelle RF Plausibilitaetspruefung ermoeglichen, ohne CRC-basierte Hauptbewertung zu ersetzen.

## Scope

- Zeitbasierte Settling-Phase nach Gain/Frequenzwechsel.
- Optionaler RF-Probe Pfad mit Korrelation gegen Referenzton.

## Betroffene Dateien

- `crates/tetra-entities/src/phy/components/soapyio.rs`
- optional neu: `crates/tetra-entities/src/phy/components/rx_rf_probe.rs`

## Konfiguration

Vorgeschlagene Flags:

- `rf_probe_enabled` (bool)
- `rf_probe_min_db` (optional threshold)
- `pll_lock_margin_ns` (u64)

## Implementierungsschritte

1. Hardwarezeit nach Umschaltung erfassen und Settling bis `pll_lock_margin_ns` erzwingen.
2. RF-Probe Modul bauen: kurzes IQ-Fenster lesen, Referenzkorrelation berechnen, `rf_probe_db` liefern.
3. Probe-Ergebnis als diagnostisches Feld in Metrikpipeline einspeisen.
4. Optionalen Plausibilitaetsfilter erlauben (z. B. Messfenster als suspect markieren).
5. Sicherstellen: Ranking bleibt primaer CRC/4-4-basiert.

## Tests

- Unit Test fuer Korrelation und Normierung.
- Integrationstest: aktivierter/deaktivierter RF-Probe Modus.
- Timingtest fuer Settling-Fenster (keine zu fruehen Messungen).

## Abnahme

- `rf_probe_db` ist reproduzierbar und wird korrekt exportiert.
- Deaktivierter Modus beeinflusst bestehenden Ablauf nicht.

## Risiken

- Zusaetzliche Laufzeit pro Kombination.
- Gegenmassnahme: Probe kurz halten und optional machen.
