# 5-B3 Detaillierter Implementierungsplan: Testgeraete-spezifische Architektur

## Ziel

Eine erweiterbare Adapterarchitektur schaffen, damit unterschiedliche Testgeraete ohne Kernumbau integrierbar sind.

## Scope

- Abstraktes Testgeraeteinterface.
- Erste Referenzimplementierung fuer Stabilock 4032.

## Betroffene Dateien

- neu: `crates/tetra-entities/src/phy/components/rx_test_devices.rs`
- `bins/bluestation-bs/src/main.rs`

## Interface Design

Vorgeschlagene Trait-Methoden:

- `device_name()`
- `burst_mode()`
- `metadata()`
- `validate_for_run()`

Optional:

- `required_slots()`
- `supports_profile(profile)`

## Implementierungsschritte

1. Trait und gemeinsame Datentypen definieren.
2. `Stabilock4032Adapter` implementieren.
3. Trennung in `operation_profile` und `signal_profile` als eigene Typen.
4. Auswahladapter ueber Config/CLI in `main.rs` verdrahten.
5. Adaptermetadaten in Export/Report durchreichen.

## Tests

- Unit Test Adapter-Validierung.
- Integrationstest mit Stabilock-Adapter und Profilwechsel.
- Negativtest fuer nicht unterstuetztes Signalprofil.

## Abnahme

- Kernlogik laeuft unveraendert mit Adapterabstraktion.
- Neuer Adapter kann ohne Aenderung in Sweep-State-Machine hinzugefuegt werden.
