# 5-D1 Detaillierter Implementierungsplan: Definierter Testablauf (Operator)

## Ziel

Einen standardisierten Laborablauf beschreiben, der wiederholbare Sensitivitaetsmessungen liefert.

## Scope

- Operator-Schritte fuer T1/SCH/TCH Tests.
- Pegelreihen und Wiederholungen.

## Ablaufprotokoll

1. Stabilock Profil fixieren (z. B. TCH/7.2 oder SCH/F).
2. BlueStation im RX-Testmodus starten.
3. Pflichtmetadaten erfassen (`test_tx_power_dbm`, `test_level_dbm`).
4. Sweep je Pegelpunkt ausfuehren.
5. Fuer T1 Pegel bis zum Verlust von 4/4 Stabilitaet reduzieren.
6. Messreihe auf- und absteigend wiederholen.

## Operative Vorgaben

- Pro Kombination mindestens 500 Bursts (empfohlen).
- Pegelschritte typischerweise 1 bis 2 dB.
- Konstante Testkonfiguration innerhalb einer Serie.

## Artefakte pro Lauf

- Rohmetriken (CSV/JSON)
- Abschlussreport
- dokumentierter Pegelpunkt

## Abnahme

- Ein externer Operator kann den Ablauf ohne Zusatzwissen reproduzieren.
