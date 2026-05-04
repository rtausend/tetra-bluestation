# Implementierungsplan: RX-Gain-Tuning mit Stabilock-Testbursts fuer tetra-bluestation

## 1. Ziel

Ziel ist ein reproduzierbarer Mess- und Auswertepfad, mit dem fuer definierte Stabilock-Testaussendungen bestimmt werden kann:

- welche RX-Gain-Einstellungen (z. B. LNA/TIA/PGA bei Lime) die beste Dekodierqualitaet liefern;
- bis zu welcher eingespeisten Sendeleistung (bzw. empfangenen Pegellage) Bursts noch sinnvoll verarbeitbar sind;
- in welchem Bereich Uebersteuerung, False Positives oder CRC-Fehler dominant werden.

Der Plan fokussiert auf den vorhandenen BS-Uplink-Pfad und baut auf bestehendem Logging/Decoding auf.

## 2. Standardbezug (arbeitsrelevant)

Fuer die Messlogik werden die folgenden TETRA-Air-Interface-Grundlagen genutzt (EN 300 392-2, in der Codebasis bereits indirekt referenziert):

- TDMA mit 4 Slots pro Frame, 18 Frames pro Multiframe, 255 Symbole pro Slot.
- pi/4-DQPSK, 18 ksps Symbolrate.
- Training-Sequenzen als primaeres Burst-Erkennungsmerkmal.
- Uplink-Burstformen:
  - Normal Uplink Burst (NUB, full slot, NormalTrainSeq1/2),
  - Control Uplink Burst (CUB, subslot, ExtendedTrainSeq).
- Qualitaetsentscheidung nicht nur auf L1-Erkennung, sondern bis L2/LMAC-Pruefung (CRC/FEC) durchziehen.

Wichtig fuer den Messaufbau: Ein "TrainingSequence gefunden" im PHY reicht alleine nicht als "gut". Fuer Gain-Tuning ist die End-to-End-Quote bis CRC entscheidend.

## 2.1 Stabilock-4032-relevante Befunde (aus Hauptmanual + TETRA-BS-Testdokument)

Aus deiner bereitgestellten Datei `M_4032_GER_0306_622_A.pdf` (Haupt-Bedienungsanleitung) konnten folgende Punkte eindeutig bestaetigt werden:

- Das Geraet fuehrt TETRA als Software-Optionen explizit auf:
  - `Tetra MS Test`
  - `Tetra BS Test`
- Das Bedienkonzept fuer Empfaengertests basiert auf:
  - `RX-Maske` (Grundmaske fuer Empfaengermessungen),
  - `RX-SPECIALS` (automatisierte typische Empfaengermessungen),
  - `GEN_A`-Umschaltung auf RX/TX-Signalweg,
  - Parameter-/Konfigurationsmasken via `DEF.PAR`.
- Die Hauptanleitung beschreibt damit den operativen Rahmen, aber nicht die vollstaendigen TETRA-Burstdetails des BS-Test-Optionspakets.

Fuer die TETRA-BS-spezifischen Burstmodi wurde zusaetzlich das TETRA-BS-Test-Supplement herangezogen:

Aus dem verfuegbaren TETRA-BS-Test-Supplement fuer 4031/4032 (Option 897 942, "TETRA BS Test") sind fuer unseren RX-Testmodus besonders relevant:

- RX-Tests auf BS werden explizit unterstuetzt ("feed a test signal to the BS, allowing to test the receiver").
- Der erzeugte RX-Testsignaltyp ist waehlbar:
  - TCH/7.2 (bursted),
  - SCH/F (bursted),
  - Bit Pattern (continuous: 0000, 1111, 0101, 1010, 1100, PN9, unmod).
- Fuer TETRA werden im Datenblatt genannt:
  - Symbolrate 18 ksym/s,
  - pi/4 DQPSK,
  - Burstmuster T1:TCH/7.2 und T1:SCH/F,
  - kontinuierliche Muster inkl. PN9.

Konsequenz fuer BlueStation:

- Der Sweep-Workflow soll burstbasierte und kontinuierliche Testmodi explizit unterscheiden.
- Fuer die eigentliche Empfindlichkeits-/Gain-Optimierung sind bursted Modi (TCH/7.2 oder SCH/F) primaer, weil sie dem realen Uplink-Verhalten naeher sind.
- Fuer T1-Testmode mit Last auf allen 4 UL-Slots muss die Auswertung slotweise erfolgen, nicht nur aggregiert.

Mapping fuer die Auswertung (wichtig fuer Konsistenz):

- Stabilock `TCH/7.2` (bursted) -> in BlueStation primaer NUB/fullslot-Pfad (Traffic).
- Stabilock `SCH/F` (bursted) -> in BlueStation primaer NUB/fullslot-Pfad (Control).
- Stabilock `Bit Pattern` (continuous, z. B. PN9) -> nur eingeschraenkt fuer End-to-End-CRC geeignet; primaer fuer RF/Modulationsbeobachtung.

Praktische Umsetzungsregel aus dem Quellenabgleich:

- Hauptmanual (`M_4032_GER_0306_622_A.pdf`) fuer Bedien- und Ablaufkonzept (RX-Maske, Specials, Signalweg).
- TETRA-BS-Supplement fuer signal-/burstspezifische Parameter (TCH/7.2, SCH/F, PN9, Autosync-Verhalten).

## 2.2 Externer Technikhinweis aus `sxxcvr`-Beispielskript

Aus dem verlinkten Skript `plot_rxtx_response.py` (tejeez/sxxcvr) sind fuer unsere Umsetzung zwei technische Muster interessant:

- Nach Frequenzwechsel explizite Wartephase bis PLL-Lock (zeitbasiert ueber Hardware-Zeit).
- Schnelle RF-Amplitudenbewertung per Korrelationsmessung gegen bekannten Ton (windowed matched-correlation).

Einordnung fuer BlueStation:

- Das ist sinnvoll als optionaler RF-Vortest/Health-Check pro Gain-Kombination.
- Fuer die eigentliche TETRA-RX-Bewertung bleibt End-to-End (Detect -> Decode -> CRC) das Primatkriterium.
- Der Korrelationswert darf in BlueStation nur Sekundaermetrik sein, nicht Ersatz fuer 4/4- bzw. CRC-Kriterien.

## 3. Ist-Stand im Projekt (Code-Mapping)

### 3.1 PHY / Burst-Erkennung

- Datei: `crates/tetra-entities/src/phy/components/demodulator.rs`
  - `SlotBurstFinder` erkennt Training-Sequenzen per Hamming-Distanz.
  - Harte Schwellwerte aktuell sehr streng (`SEQ_*_MAX_ERRS = 1`).
  - Intern vorhanden: `train_errs`, `burst_pos`, `train_type`.

- Datei: `crates/tetra-entities/src/phy/phy_bs.rs`
  - `rx_tpsap_prim()` loggt aktuell nur Ereignisse wie:
    - `got NormalTrainSeq1 in fullslot`
  - Bursts werden an LMAC weitergereicht (`split_rxslot_and_send_to_lmac`).

### 3.2 SDR/Gain-Konfiguration

- Datei: `crates/tetra-config/src/bluestation/sec_phy.rs`
  - Liest `rx_gain_*` und `tx_gain_*` aus `config.toml` ein.

- Datei: `crates/tetra-entities/src/phy/components/soapy_settings.rs`
  - Device-spezifische Gain-Elemente und Defaults (z. B. Lime: LNA/TIA/PGA).

- Datei: `crates/tetra-entities/src/phy/components/soapyio.rs`
  - Setzt Gains via `set_gain_element(...)` beim Start.
  - Laufzeit-Aenderung der RX-Gains ist aktuell nicht vorgesehen.

### 3.3 LMAC-Decoding als Qualitaetskriterium

- Datei: `crates/tetra-entities/src/lmac/lmac_bs.rs`
  - `rx_blk_control()` verwendet `decode_cp(...)` und prueft `crc_pass`.
  - `rx_blk_traffic()` verwendet `decode_tp(...)` und bewertet `crc_ok`.

- Datei: `crates/tetra-entities/src/lmac/components/errorcontrol.rs`
  - `decode_cp(...)` und `decode_tp(...)` liefern CRC-Erfolg.

## 4. Kernidee fuer die Auswertung

Wir bewerten jede Gain/Pegel-Kombination ueber eine Metrik-Kaskade:

1. **L1 Detect Rate**
  - Anteil der detektierten Bursts relativ zu `expected_slots` im Messfenster.
2. **L1 Train Quality**
   - Verteilung `train_errs` und Positionsoffset `burst_pos`.
3. **L2 Decode Rate (Control/Traffic)**
   - Anteil der Bursts, die bis `decode_cp/decode_tp` gelangen.
4. **CRC Pass Rate**
   - Anteil mit `crc_pass == true` bzw. `crc_ok == true`.
5. **False Positive Rate**
   - Bursts mit Training-Erkennung, aber ohne valide Folgeverarbeitung.

Hinweis zur Metrik-Definition:

- `expected_slots` wird im Testlauf als Anzahl betrachteter Slotgelegenheiten im Messfenster gefuehrt (nicht als extern garantierte Burstrate des Testgeraets). Damit bleibt die Kennzahl auch bei variablen Burstmustern konsistent.

T1-spezifische Erweiterung (4/4-Kriterium):

- Fuer T1-Tests wird zusaetzlich je Timeslot `slot_crc_pass_rate[0..3]` ausgewertet.
- Ein Pegel gilt als "4/4 stabil", wenn alle 4 UL-Slots den konfigurierten Mindestwert erreichen (z. B. `slot_crc_pass_rate >= 95%`).
- Die "untere Empfindlichkeitsgrenze" fuer ein Gain-Set ist der niedrigste Pegel, bei dem "4/4 stabil" noch gilt.

Der fuer den Betrieb beste Gain liegt in dem Bereich, in dem CRC-Pass maximal ist, bei gleichzeitig niedriger False-Positive-Rate und ohne Clipping-Indizien.

## 5. Implementierungsplan

## Phase A - Telemetrie erweitern (ohne Verhaltensaenderung)

### A1. PHY-Debugdaten pro Burst sichtbar machen

Dateien:

- `crates/tetra-entities/src/phy/components/demodulator.rs`
- `crates/tetra-entities/src/phy/phy_bs.rs`

Schritte:

- `SlotBurstFinder` um lesbare Burst-Metadaten erweitern (mindestens: `train_errs`, `burst_pos`, `burst_len`).
- Diese Metadaten in `RxBurstBits`/neuem Meta-Struct bis `phy_bs.rs` propagieren.
- Logging in `rx_tpsap_prim()` von reinem "train_type gefunden" auf strukturierte Felder erweitern:
  - timestamp/tdma time,
  - fullslot/subslot1/subslot2,
  - training sequence,
  - train_errs,
  - burst_pos.

Ergebnis:

- Sichtbar, ob Gain den Korrelationsabstand verschlechtert, lange bevor CRC kippt.

### A2. LMAC-Erfolgsmetriken konsistent loggen

Dateien:

- `crates/tetra-entities/src/lmac/lmac_bs.rs`
- optional `crates/tetra-entities/src/lmac/components/errorcontrol.rs`

Schritte:

- In `rx_blk_control()` und `rx_blk_traffic()` standardisierte Messlogs ausgeben:
  - logical channel,
  - block_num,
  - decode attempted (ja/nein),
  - crc result.
- Eindeutige Korrelation zum PHY-Ereignis (z. B. ueber tdma time + slot type).

Ergebnis:

- End-to-End-Messung von Detektion bis CRC.

### A3. Optional: RF-Vorqualifizierung pro Gain-Kombination

Dateien:

- `crates/tetra-entities/src/phy/components/soapyio.rs`
- optional neue Komponente unter `crates/tetra-entities/src/phy/components/` (z. B. `rx_rf_probe.rs`)

Schritte:

- Nach Gain- oder Frequenzwechsel eine kurze zeitbasierte Settling-Phase auf Basis von Hardware-Zeit einbauen (zusaetzlich zu Slot-basiertem Settling).
- Optionalen RF-Probe-Modus vor dem eigentlichen Messfenster anbieten:
  - kurzes I/Q-Fenster erfassen,
  - gegen Referenzton korrelieren,
  - normierten Korrelationspegel als `rf_probe_db` loggen.
- `rf_probe_db` nur als Diagnostik verwenden (z. B. zur Erkennung von offensichtlichem Clipping/Untersteuerung), nicht als Ranking-Hauptkriterium.

Ergebnis:

- Schnellere Plausibilitaetspruefung und robustere Messfensterstarts nach Umschaltvorgaengen.

## Phase B - Messmodus fuer Gain-Sweeps

### B0. Eindeutiger RX-Test-Startmodus (Schalter)

Dateien:

- `bins/bluestation-bs/src/main.rs`
- `crates/tetra-config/src/bluestation/config.rs`

Schritte:

- Expliziten Programm-Schalter fuer RX-Testmodus einfuehren, z. B. `--rx-gain-test`.
- Ohne Schalter laeuft BlueStation unveraendert im normalen Betriebsmodus.
- Mit Schalter wird ein klar separater Testablauf aktiviert:
  - Sweep starten,
  - Messfenster abarbeiten,
  - Ergebnisreport ausgeben,
  - Prozess mit Exit-Code 0 sauber beenden.
- Fehlerfaelle (ungueltige Konfiguration, keine gueltigen Gain-Elemente, keine Samples) mit nicht-0 Exit-Code beenden.

Ergebnis:

- Verwechselung mit normalem Betrieb wird vermieden; Testlauf ist reproduzierbar und eindeutig.

### B1. Range-basierte Gain-Sweep-Konfiguration

Dateien:

- `crates/tetra-config/src/bluestation/sec_phy_soapy.rs`
- `crates/tetra-config/src/bluestation/sec_phy.rs`
- `example_config/config.toml`

Schritte:

- Optionalen Messmodus einfuehren, z. B. `rx_gain_sweep` mit Bereichsangaben pro Gain-Element.
- Pro vorhandenem Gain-Element (geraeteabhaengig) optionalen Bereich erlauben:
  - `from`,
  - `to`,
  - `step`.
- Beispielidee (konzeptionell):
  - nur `pga` vorhanden: Sweep nur ueber `pga`.
  - `lna` und `pga` vorhanden: Sweep ueber beide Bereiche (kartesisches Produkt).
- Optionalen Modus fuer Suchstrategie anbieten:
  - `grid` (alle Kombinationen),
  - spaeter optional `coordinate_descent` (schneller bei grossem Suchraum).
- Rueckwaertskompatibel halten: wenn `rx_gain_sweep` nicht gesetzt ist, heutiges statisches Verhalten mit `rx_gain_*`.

Vorgeschlagenes Konfigurationsschema:

```toml
[phy_io.soapysdr.rx_gain_sweep]
enabled = true
strategy = "grid"         # optional, default: grid
window_bursts = 500        # Bursts pro Messfenster
settling_slots = 8         # zu verwerfende Slots nach Gainwechsel
auto_exit = true           # Testmodus beendet Prozess nach Abschlussreport

# Testsignal-Charakteristik fuer die Auswertung
test_signal_profile = "t1_all_ul_slots"  # z. B. t1_all_ul_slots, t1_single_slot
required_ul_slots = [0, 1, 2, 3]
min_slot_crc_pass_rate = 0.95

# Optional: welche Testgeraeteleistung fuer diesen Lauf angenommen wird
# (wird im Ergebnisreport mit ausgegeben)
test_tx_power_dbm = -85.0

# Testpegel-Label fuer diesen Lauf (Pegel am DUT-Eingang)
# Ein Run == ein Pegelpunkt; mehrere Pegelpunkte werden als mehrere Runs gefahren.
test_level_dbm = -95.0

# Nur die Gains angeben, die das Device wirklich hat
[phy_io.soapysdr.rx_gain_sweep.gains.pga]
from = 0.0
to = 30.0
step = 1.0

[phy_io.soapysdr.rx_gain_sweep.gains.lna]
from = 0.0
to = 30.0
step = 3.0
```

Ergebnis:

- Automatische Erzeugung der zu testenden Gain-Kombinationen aus Bereichsangaben.

### B1.1 Testgeraeteleistung erfassen (Config oder Eingabe)

Dateien:

- `bins/bluestation-bs/src/main.rs`
- `crates/tetra-config/src/bluestation/sec_phy_soapy.rs`

Schritte:

- Testgeraeteleistung als Metadatum verpflichtend fuer den RX-Testlauf machen.
- Zusaetzlich `test_level_dbm` (effektiver Pegel am DUT-Eingang) als Pflichtmetadatum fuehren.
- Prioritaet der Quelle:
  1. CLI-Argument (z. B. `--test-tx-power-db`, `--test-level-db`),
  2. Konfigurationswert (`test_tx_power_dbm`, `test_level_dbm`).
- Falls keine Quelle verfuegbar ist: Start des Testlaufs mit klarer Fehlermeldung abbrechen.
- Keine interaktive Eingabe waehrend eines gestarteten Testlaufs (autarker Betrieb).

Ergebnis:

- Jeder Lauf ist auf einen dokumentierten Sendeleistungswert rueckfuehrbar.
- Jeder Lauf ist eindeutig einem Pegelpunkt zugeordnet.

### B2. Laufzeitwechsel der RX-Gains

Dateien:

- `crates/tetra-entities/src/phy/components/soapyio.rs`
- `crates/tetra-entities/src/phy/components/soapy_dev.rs`

Schritte:

- Interne Runtime-Schnittstelle fuer kontrolliertes Umschalten der RX-Gain-Kombinationen zwischen Messfenstern bauen (kein externes User-API).
- Wechsel nur an sicheren Grenzen (z. B. Slotgrenzen) und mit Settling-Guard (x Slots verwerfen).
- Sweep-State-Machine einfuehren:
  - `ApplyGains`,
  - `Settling`,
  - `MeasureWindow`,
  - `FinalizeAndNext`.
- Pro Kombination Messfenster mit fixer Burstanzahl auswerten (`window_bursts`).
- Nach Start des Testmodus laeuft die State-Machine vollautomatisch bis Abschlussreport und Prozessende.
- Nach Sweep-Ende automatische Bestwert-Ermittlung:
  - primaer nach `crc_pass_rate`,
  - bei T1 `required_ul_slots=[0,1,2,3]` zusaetzlich nur Kandidaten zulassen, die 4/4 stabil sind,
  - bei Gleichstand nach geringerer `false_positive_rate`,
  - optional zusaetzlich nach niedrigerem mittleren `train_errs`,
  - `rf_probe_db` nur als Diagnose-/Plausibilitaetsfilter (optional), nicht als Primaerranking.
- Bestes Gain-Set als Ergebnislog ausgeben; optional kann es zusaetzlich in eine Ergebnisdatei fuer den naechsten Start geschrieben werden.

Ergebnis:

- Vollautomatischer Sweep ohne Prozessneustart, inklusive automatischer Auswahl des besten Gain-Sets.

### B3. Testgeraete-spezifische Architektur (erweiterbar)

Dateien:

- neue Komponente, z. B. `crates/tetra-entities/src/phy/components/rx_test_devices.rs`
- `bins/bluestation-bs/src/main.rs`

Schritte:

- Abstraktes Interface fuer Testgeraete definieren, z. B.:
  - `device_name()`,
  - `burst_mode()` (bursted/continuous),
  - `metadata()` (z. B. tx_power_dbm, pattern, ggf. slot info),
  - `validate_for_run()`.
- Erste Implementierung: `Stabilock4032Adapter`.
- Der `Stabilock4032Adapter` soll zwei Ebenen trennen:
  - `operation_profile` (aus Hauptmanual: RX-Maske/Special-Logik),
  - `signal_profile` (aus TETRA-BS-Supplement: TCH/7.2, SCH/F, Bit Pattern/PN9).
- Defaultpfad erlaubt spaeter weitere Adapter (z. B. R&S, Aeroflex, Keysight) ohne Eingriff in Sweep-Kernlogik.
- Ergebnisreport speichert auch `test_device_type` und `test_signal_mode`.

Ergebnis:

- Andere Testgeraete koennen durch neue Adapterklassen sauber erweitert werden.

## Phase C - Aggregation und Ergebnisdatei

### C1. In-Memory-Statistik

Dateien:

- `crates/tetra-entities/src/phy/phy_bs.rs`
- optional neue Komponente unter `crates/tetra-entities/src/phy/components/`

Schritte:

- Counter je (gain_combo, test_level_dbm, burst_type, train_type) aufbauen:
  - expected_slots,
  - detected,
  - decoded,
  - crc_ok,
  - false_positive.
- Rolling-Window fuer schnelle Sicht + kumulativ fuer Abschlussbericht.
- Zusaetzlich slotweise Counter fuer UL-Slots 0..3 fuehren, damit 4/4-Stabilitaet auswertbar ist.

### C2. Export fuer Offline-Auswertung

Dateien:

- neue Datei, z. B. `crates/tetra-entities/src/phy/components/rx_gain_stats.rs`

Schritte:

- Periodischer CSV/JSON-Export (maschinenlesbar), z. B.:
  - `timestamp, gain_combo, test_level_dbm, burst, detected, crc_ok_rate, slot0_crc_rate, slot1_crc_rate, slot2_crc_rate, slot3_crc_rate, train_err_mean, fp_rate, rf_probe_db, test_device_type, test_signal_mode, test_tx_power_dbm`.
- Zusaetzlich Abschluss-Ranking exportieren, z. B.:
  - `rank, gain_combo, crc_ok_rate, fp_rate, train_err_mean, samples`.

Ergebnis:

- Plotbar in externen Tools und reproduzierbar fuer Regressionen.

### C3. Abschlussreport und Prozessende

Dateien:

- `bins/bluestation-bs/src/main.rs`
- optional neue Utility-Datei fuer Reportformatierung

Schritte:

- Nach Ende aller Kombinationen einen kompakten Abschlussreport ausgeben:
  - bestes Gain-Set,
  - Top-3-Ranking,
  - gemessene Testgeraeteleistung,
  - Burstmodus,
  - Anzahl Samples und Qualitaetskennzahlen,
  - 4/4-Bewertung je Pegelpunkt (pass/fail) und niedrigster 4/4-stabiler Pegel.
- Danach im RX-Testmodus Prozess kontrolliert beenden.

Ergebnis:

- Lauf ist abgeschlossen, Ergebnis direkt sichtbar, keine manuelle Nacharbeit noetig.

## Phase D - Operator-Workflow (Stabilock)

### D1. Definierter Testablauf

1. Stabilock auf festen Bursttyp und feste Rate konfigurieren (z. B. TCH/7.2 oder SCH/F).
2. BlueStation im expliziten RX-Testmodus starten (CLI-Schalter gesetzt).
3. Testgeraeteleistung erfassen (CLI, Config oder Eingabe).
4. Pegelpunkt (`test_level_dbm`) fuer diesen Lauf setzen.
5. Innerhalb dieses Pegelpunkts:
  - alle automatisch erzeugten Gain-Kombinationen durchlaufen,
  - pro Kombination mindestens N Bursts sammeln (empfohlen >= 500).
6. Bei T1-Mode den Pegel schrittweise reduzieren (z. B. von -40 dBm bis in den Bereich um -137 dBm), bis 4/4 nicht mehr stabil erreicht wird.
7. Nach Laufende Abschlussreport lesen; Prozess beendet sich automatisch.
8. Fuer weitere Pegelpunkte (z. B. 1 oder 2 dB Schritte) Testgeraet umstellen und Run wiederholen.
9. Messung fuer steigende und fallende Pegel wiederholen (Hysterese/Saettigung erkennen).

### D2. Near-Far-Szenario (Praxisfall) explizit pruefen

Ziel:

- Robustheit gegen gleichzeitige starke und schwache UL-Signale auf unterschiedlichen Timeslots messen.

Vorgeschlagener Test:

1. Timeslot A mit hohem Pegel einspeisen (nahes MS-Aequivalent).
2. Timeslot B mit deutlich niedrigerem Pegel einspeisen (fernes MS-Aequivalent).
3. Pegeldifferenz in Stufen vergroessern (z. B. 10, 20, 30 dB), bei konstantem mittleren Gesamtpegel.
4. Pro Stufe slotweise Decode/CRC-Raten erfassen und auf Ausfall des schwachen Slots pruefen.

Ergebnis:

- Dynamikbereichsgrenze (near-far margin) als zusaetzliche Betriebskennzahl verfuegbar.

### D3. Entscheidungsregeln

Empfohlene Primarziele:

- `crc_pass_rate` maximieren,
- `false_positive_rate` minimieren,
- stabiler Bereich mit geringer Streuung von `train_errs`.

Sekundaer:

- grossmoeglicher nutzbarer Pegelbereich (untere Empfindlichkeitsgrenze bis obere Uebersteuerungsgrenze).

## 6. Definition "sinnvoll verarbeitbar"

Pragmatische technische Definition fuer den Report:

- **Control Bursts**: sinnvoll verarbeitbar, wenn `crc_pass_rate >= 95%` ueber Messfenster.
- **Traffic Bursts**: sinnvoll verarbeitbar, wenn `decode_tp` stabil und `crc_ok` im Zielbereich (projektspezifisch festlegen, z. B. >= 90%).
- **Nicht sinnvoll**: wenn nur Training erkannt wird, aber CRC-Quote deutlich abfaellt oder False Positives zunehmen.
- **T1 all-UL-slots**: sinnvoll verarbeitbar nur, wenn alle geforderten UL-Slots den Mindest-CRC-Wert halten (4/4-Kriterium).

Hinweis: Schwellwerte bewusst als Konfigurationsparameter umsetzbar machen.

## 7. Test- und Abnahmeplan

- Unit-Tests fuer neue Statistik- und Exportlogik.
- Integrationstest mit `ul_input_file` (deterministischer Replay-Pfad) fuer reproduzierbare Vergleiche.
- Feldtest mit Stabilock:
  - mindestens 2 komplette Sweep-Durchlaeufe,
  - Vergleich der Top-2-Profile auf Wiederholbarkeit.
- Near-Far-Test:
  - mindestens ein Lauf mit zwei gleichzeitig aktiven UL-Slots und >= 20 dB Pegeldifferenz,
  - Nachweis, ab welcher Differenz der schwache Slot kippt.

Abnahme, wenn:

- automatischer Sweep ohne Crash laeuft,
- Ergebnisdatei pro Sweep erzeugt wird,
- klarer "best profile" und belastbare Pegelgrenzen auswertbar sind.

## 8. Risiken und Gegenmassnahmen

- **Zu strenge Detektionsschwellen (`SEQ_*_MAX_ERRS = 1`)**:
  - Gegenmassnahme: Schwellwerte im Messmodus parametrierbar machen und separat evaluieren.
- **Gain-Umschalttransienten**:
  - Gegenmassnahme: Settling-Slots verwerfen.
- **Nichtlineare Frontend-Effekte bei hohen Pegeln**:
  - Gegenmassnahme: zusaetzlich train_errs/burst_pos drift beobachten, nicht nur CRC.
- **Near-Far-Ueberdeckung (starker Slot verdrangt schwachen Slot)**:
  - Gegenmassnahme: slotweise Kennzahlen und separaten Near-Far-Abnahmetest verpflichtend machen.
- **Unterschied zwischen Laborsetup und Realwelt**:
  - Gegenmassnahme: Top-Profile mit realem OTA-Signal gegenpruefen.
- **Zu grosser Suchraum (viele Gain-Kombinationen)**:
  - Gegenmassnahme: maximale Kombinationszahl konfigurierbar begrenzen und optional auf schnellere Suchstrategie (z. B. coordinate descent) umschalten.

## 9. Umsetzungsreihenfolge (empfohlen)

1. Phase A (Telemetry) - sofortiger Mehrwert, geringes Risiko.
2. Phase B (Testmodus + Sweep-Infrastruktur) - Laufsteuerung und Gainwechsel bereitstellen.
3. Phase C (Aggregation/Export) - Kennzahlen und Ranking auf stabilem Ablauf aufbauen.
4. Phase D (operatorische Dokumentation + Feinschliff).

## 10. Erwartetes Ergebnis

Nach Umsetzung liegt ein reproduzierbarer RX-Tuning-Workflow vor, mit dem fuer Stabilock-Testbursts objektiv beantwortet werden kann:

- Welche RX-Gain-Kombination ist optimal?
- Wo liegt die untere Empfindlichkeitsgrenze?
- Ab welchem Pegel beginnen Uebersteuerung oder Fehlentscheidungen?

Damit wird das aktuelle Einzel-Logevent

- `rx_tpsap_prim got NormalTrainSeq1 in fullslot`

zu einer belastbaren, quantitativen Entscheidungsgrundlage fuer den Betrieb.
