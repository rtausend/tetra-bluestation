# 5-D3 Detaillierter Implementierungsplan: Entscheidungsregeln

## Ziel

Eine stabile und nachvollziehbare Regelbasis fuer Profilauswahl und Grenzentscheidungen festlegen.

## Scope

- Priorisierte Metrikregeln.
- Tie-breaker und Ausnahmeregeln.

## Regelhierarchie

1. `crc_pass_rate` maximieren.
2. Bei T1 nur 4/4 stabile Kandidaten zulassen.
3. `false_positive_rate` minimieren.
4. `train_errs` Streuung minimieren.
5. Optional `rf_probe_db` nur als Plausibilitaetsfilter.

## Grenzdefinitionen

- Control default: `crc_pass_rate >= 95%`
- Traffic default: projektspezifisch, z. B. `>= 90%`
- T1 all-slots: alle geforderten Slots >= `min_slot_crc_pass_rate`

## Implementierungsschritte

1. Regelengine als klar getrennte Funktion/Modul einfuehren.
2. Konfigurierbare Schwellwerte mit sinnvollen Defaults bereitstellen.
3. Tie-breaker deterministisch implementieren.
4. Entscheidungspfad im Abschlussreport transparent darstellen.

## Tests

- Unit Tests fuer alle Rankingpfade.
- Grenzfalltests bei Gleichstand.
- Regressionstest mit bekannten Referenzdaten.

## Abnahme

- Bestprofilauswahl ist reproduzierbar und erklaerbar.
- Operator kann jede Entscheidung auf Metrikebene nachvollziehen.
