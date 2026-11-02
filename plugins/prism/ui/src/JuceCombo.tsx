import { useEffect, useState } from "react";
import * as Juce from "juce-framework-frontend";

type Props = {
  identifier: string;
};

export function JuceCombo({ identifier }: Props) {
  const state = Juce.getComboBoxState(identifier);
  const [index, setIndex] = useState(state.getChoiceIndex());
  const [properties, setProperties] = useState(state.properties);

  useEffect(() => {
    const valueId = state.valueChangedEvent.addListener(() => {
      setIndex(state.getChoiceIndex());
    });
    const propsId = state.propertiesChangedEvent.addListener(() => {
      setProperties({ ...state.properties });
    });
    return () => {
      state.valueChangedEvent.removeListener(valueId);
      state.propertiesChangedEvent.removeListener(propsId);
    };
  }, [state]);

  const choices = properties.choices ?? [];
  const safeIndex =
    Number.isFinite(index) && choices.length > 0
      ? Math.min(Math.max(Math.round(index), 0), choices.length - 1)
      : 0;

  return (
    <div className="control">
      <label>
        <span>{properties.name || identifier}</span>
        <span className="value">{choices[safeIndex] ?? ""}</span>
      </label>
      <select
        value={safeIndex}
        onChange={(e) => {
          const next = Number(e.target.value);
          state.setChoiceIndex(next);
          setIndex(next);
        }}
      >
        {choices.map((choice, i) => (
          <option key={`${choice}-${i}`} value={i}>
            {choice}
          </option>
        ))}
      </select>
    </div>
  );
}
