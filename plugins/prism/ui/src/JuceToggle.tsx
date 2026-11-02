import { useEffect, useState } from "react";
import * as Juce from "juce-framework-frontend";

type Props = {
  identifier: string;
};

export function JuceToggle({ identifier }: Props) {
  const state = Juce.getToggleState(identifier);
  const [value, setValue] = useState(state.getValue());
  const [properties, setProperties] = useState(state.properties);

  useEffect(() => {
    const valueId = state.valueChangedEvent.addListener(() => {
      setValue(state.getValue());
    });
    const propsId = state.propertiesChangedEvent.addListener(() => {
      setProperties({ ...state.properties });
    });
    return () => {
      state.valueChangedEvent.removeListener(valueId);
      state.propertiesChangedEvent.removeListener(propsId);
    };
  }, [state]);

  return (
    <div className="control control-toggle">
      <label>
        <span>{properties.name || identifier}</span>
        <span className="value">{value ? "On" : "Off"}</span>
      </label>
      <button
        type="button"
        className={value ? "toggle on" : "toggle"}
        aria-pressed={value}
        onClick={() => {
          const next = !value;
          state.setValue(next);
          setValue(next);
        }}
      >
        {value ? "On" : "Off"}
      </button>
    </div>
  );
}
