import { useEffect, useState } from "react";
import { JuceCombo } from "./JuceCombo";
import { JuceSlider } from "./JuceSlider";
import { JuceToggle } from "./JuceToggle";
import { PitchMap } from "./PitchMap";
import * as Juce from "juce-framework-frontend";

export default function App() {
  const dryCorrectState = Juce.getToggleState("dryCorrect");
  const [dryCorrect, setDryCorrect] = useState(dryCorrectState.getValue());

  useEffect(() => {
    const id = dryCorrectState.valueChangedEvent.addListener(() => {
      setDryCorrect(dryCorrectState.getValue());
    });
    return () => dryCorrectState.valueChangedEvent.removeListener(id);
  }, [dryCorrectState]);

  return (
    <main>
      <h1>Prism</h1>
      <p className="subtitle">
        MIDI multi-voice pitch prism · shared f0 + per-note TD-PSOLA (up to 8
        notes). Voices Level is wet gain. F0 Floor down to 50 Hz (more latency;
        low floors use a cheaper decimated detector to avoid xruns).
      </p>

      <PitchMap />

      <h2 className="section-title">Mix</h2>
      <div className="grid">
        <JuceToggle identifier="dryCorrect" />
        <JuceSlider identifier="dryLevel" />
        <JuceSlider identifier="mix" />
      </div>

      <h2 className="section-title">Shared</h2>
      <div className="grid">
        <JuceSlider identifier="retuneSpeedMs" />
        <JuceSlider identifier="correctionAmount" />
        <JuceSlider identifier="vibratoPreserve" />
        <JuceSlider identifier="formantFollow" />
        <JuceSlider identifier="minF0Hz" />
        <JuceSlider identifier="referenceHz" />
      </div>

      <h2 className="section-title">Dry correct</h2>
      <div className={`grid${dryCorrect ? "" : " dimmed"}`}>
        <JuceCombo identifier="keySemitone" />
        <JuceCombo identifier="scale" />
        <JuceSlider identifier="correctionRangeSemitones" />
        <JuceSlider identifier="quantizeDeadZoneCents" />
      </div>
    </main>
  );
}
