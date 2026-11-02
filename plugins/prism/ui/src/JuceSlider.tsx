import { useEffect, useState } from "react";
import * as Juce from "juce-framework-frontend";

type Props = {
  identifier: string;
};

export function JuceSlider({ identifier }: Props) {
  const state = Juce.getSliderState(identifier);
  const [value, setValue] = useState(state.getNormalisedValue());
  const [properties, setProperties] = useState(state.properties);

  useEffect(() => {
    const valueId = state.valueChangedEvent.addListener(() => {
      setValue(state.getNormalisedValue());
    });
    const propsId = state.propertiesChangedEvent.addListener(() => {
      setProperties({ ...state.properties });
    });
    return () => {
      state.valueChangedEvent.removeListener(valueId);
      state.propertiesChangedEvent.removeListener(propsId);
    };
  }, [state]);

  const scaled = state.getScaledValue();
  const label = properties.label ? ` ${properties.label}` : "";
  const safeValue = Number.isFinite(value) ? value : 0;
  const steps = Math.max(properties.numSteps ?? 100, 2);

  return (
    <div className="control">
      <label>
        <span>{properties.name || identifier}</span>
        <span className="value">
          {Number.isFinite(scaled)
            ? Number.isInteger(scaled)
              ? scaled
              : scaled.toFixed(2)
            : "—"}
          {label}
        </span>
      </label>
      <input
        type="range"
        min={0}
        max={1}
        step={1 / (steps - 1)}
        value={safeValue}
        onMouseDown={() => state.sliderDragStarted()}
        onTouchStart={() => state.sliderDragStarted()}
        onChange={(e) => {
          const next = Number(e.target.value);
          state.setNormalisedValue(next);
          setValue(next);
        }}
        onMouseUp={() => state.sliderDragEnded()}
        onTouchEnd={() => state.sliderDragEnded()}
      />
    </div>
  );
}
