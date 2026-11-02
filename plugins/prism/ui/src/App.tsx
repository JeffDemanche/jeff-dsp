import { JuceCombo } from "./JuceCombo";
import { JuceSlider } from "./JuceSlider";

export default function App() {
  return (
    <main>
      <h1>Prism</h1>
      <p className="subtitle">
        Key-constrained pitch correct · phase 1. If Standalone is silent, open
        Options and uncheck Mute audio input.
      </p>
      <div className="grid">
        <JuceCombo identifier="keySemitone" />
        <JuceCombo identifier="scale" />
        <JuceSlider identifier="retuneSpeedMs" />
        <JuceSlider identifier="correctionAmount" />
        <JuceSlider identifier="correctionRangeSemitones" />
        <JuceSlider identifier="vibratoPreserve" />
        <JuceSlider identifier="formantFollow" />
        <JuceSlider identifier="quantizeDeadZoneCents" />
        <JuceSlider identifier="mix" />
        <JuceSlider identifier="minF0Hz" />
        <JuceSlider identifier="referenceHz" />
      </div>
    </main>
  );
}
