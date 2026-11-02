/// <reference types="vite/client" />

declare module "juce-framework-frontend" {
  export interface SliderState {
    getNormalisedValue(): number;
    setNormalisedValue(value: number): void;
    getScaledValue(): number;
    sliderDragStarted(): void;
    sliderDragEnded(): void;
    properties: {
      name: string;
      label: string;
      numSteps: number;
      parameterIndex: number;
    };
    valueChangedEvent: {
      addListener(cb: () => void): number;
      removeListener(id: number): void;
    };
    propertiesChangedEvent: {
      addListener(cb: () => void): number;
      removeListener(id: number): void;
    };
  }

  export interface ComboBoxState {
    getChoiceIndex(): number;
    setChoiceIndex(index: number): void;
    properties: {
      name: string;
      choices: string[];
      parameterIndex: number;
    };
    valueChangedEvent: {
      addListener(cb: () => void): number;
      removeListener(id: number): void;
    };
    propertiesChangedEvent: {
      addListener(cb: () => void): number;
      removeListener(id: number): void;
    };
  }

  export function getSliderState(id: string): SliderState;
  export function getComboBoxState(id: string): ComboBoxState;
}
