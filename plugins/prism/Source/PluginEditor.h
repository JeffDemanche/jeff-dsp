#pragma once

#include "PluginProcessor.h"

class PrismAudioProcessorEditor final : public juce::AudioProcessorEditor
{
public:
    explicit PrismAudioProcessorEditor (PrismAudioProcessor&);
    ~PrismAudioProcessorEditor() override = default;

    void paint (juce::Graphics&) override;
    void resized() override;

private:
    PrismAudioProcessor& audioProcessor;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (PrismAudioProcessorEditor)
};
