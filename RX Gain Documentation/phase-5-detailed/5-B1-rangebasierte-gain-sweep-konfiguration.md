# 5-B1 Detaillierter Implementierungsplan: Rangebasierte Gain Sweep Konfiguration

## Ziel

Aus deklarativen Bereichen pro Gain-Element automatisch Sweep-Kombinationen erzeugen.

## Scope

- Neues Config-Schema `rx_gain_sweep`.
- Strategieauswahl (`grid`, spaeter `coordinate_descent`).

## Betroffene Dateien

- `crates/tetra-config/src/bluestation/sec_phy_soapy.rs`
- `crates/tetra-config/src/bluestation/sec_phy.rs`
- `example_config/config.toml`

## Datenmodell

Neue Typen:

- `RxGainSweepConfig`
- `GainRange { from, to, step }`
- `SweepStrategy`

Validierungsregeln:

- `step > 0`
- `from <= to`
- nur bekannte Gain-Namen pro Device
- Kombinationslimit optional gegen Suchraumexplosion

## Implementierungsschritte

1. Parsingmodelle in `sec_phy_soapy.rs` einfuehren.
2. Parser in `sec_phy.rs` integrieren und Rueckwaertskompatibilitaet absichern.
3. Funktion zur Kombinationsgenerierung implementieren (cartesian product).
4. Sortierung und deterministische Reihenfolge der Kombinationen sicherstellen.
5. Beispielconfig erweitern und kommentieren.

## Tests

- Unit Tests fuer Parsing gueltig/ungueltig.
- Unit Tests fuer Kombinationserzeugung.
- Regressionstest: wenn `rx_gain_sweep` fehlt, bleibt Altverhalten.

## Abnahme

- Sweep-Kombinationen stimmen mit Konfigurationsbereichen ueberein.
- Ungueltige Config liefert klare Fehlermeldung.
