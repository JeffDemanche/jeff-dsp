#include "PluginEditor.h"

PrismAudioProcessorEditor::PrismAudioProcessorEditor (PrismAudioProcessor& p)
    : AudioProcessorEditor (&p),
      audioProcessor (p)
{
    juce::ignoreUnused (audioProcessor);
    setSize (420, 240);
}

void PrismAudioProcessorEditor::paint (juce::Graphics& g)
{
    g.fillAll (juce::Colour (0xff12141a));
    g.setColour (juce::Colour (0xffe8ecf4));
    g.setFont (juce::FontOptions (28.0f, juce::Font::bold));
    g.drawFittedText ("Prism", getLocalBounds().reduced (24, 48),
                      juce::Justification::centredTop, 1);

    g.setFont (juce::FontOptions (14.0f));
    g.setColour (juce::Colour (0xff9aa3b5));
    g.drawFittedText ("JUCE shell + Rust DSP skeleton",
                      getLocalBounds().reduced (24, 90),
                      juce::Justification::centredTop, 1);
}

void PrismAudioProcessorEditor::resized()
{
}
