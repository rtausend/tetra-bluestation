# 5-D2 Detaillierter Implementierungsplan: Near-Far Szenario

## Ziel

Robustheit gegen gleichzeitige starke und schwache UL-Signale quantifizieren und als Margin dokumentieren.

## Scope

- Zwei gleichzeitige UL-Slots mit unterschiedlichem Pegel.
- Slotweise CRC/Decode-Auswertung.

## Testdesign

Grundfall:

- Slot A: hoher Pegel (near)
- Slot B: niedriger Pegel (far)

Differenzstufen:

- 10 dB
- 20 dB
- 30 dB
- optional feiner im Kippbereich

## Implementierungsschritte

1. Testprofil fuer zwei aktive Slots definieren.
2. Pro Differenzstufe stabile Messfenster erfassen.
3. Slotweise Kennzahlen berechnen (`slot_crc_pass_rate`).
4. Kippkriterium des schwachen Slots definieren (z. B. CRC < threshold).
5. Near-Far-Margin pro Gainprofil reporten.

## Auswertungsregeln

- Primaer relevant: schwacher Slot.
- Starker Slot dient als Referenz fuer potenzielle Ueberdeckung.

## Tests

- Integrationstest mit synthetischem Replay (falls moeglich).
- Feldtest mit realem Signalgenerator.

## Abnahme

- Es liegt eine dokumentierte Marginkurve ueber Pegeldifferenz vor.
- Kippbereich ist fuer Betriebsempfehlung nutzbar.
