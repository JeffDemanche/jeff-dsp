#pragma once

#include "PluginProcessor.h"

#include <juce_gui_extra/juce_gui_extra.h>

class PrismAudioProcessorEditor final : public juce::AudioProcessorEditor,
                                        private juce::Timer
{
public:
    explicit PrismAudioProcessorEditor (PrismAudioProcessor&);
    ~PrismAudioProcessorEditor() override = default;

    void paint (juce::Graphics&) override;
    void resized() override;

private:
    void timerCallback() override;

    std::optional<juce::WebBrowserComponent::Resource> getResource (const juce::String& url) const;
    juce::String resolveInitialUrl() const;
    juce::var meteringToVar (const PrismDspMetering& m) const;

    struct SinglePageBrowser final : juce::WebBrowserComponent
    {
        using juce::WebBrowserComponent::WebBrowserComponent;

        std::function<void()> onPageFinished;

        void pageFinishedLoading (const juce::String&) override
        {
            if (onPageFinished)
                onPageFinished();
        }

        bool pageAboutToLoad (const juce::String& newURL) override
        {
            const auto root = juce::WebBrowserComponent::getResourceProviderRoot();
            if (newURL == root || newURL.startsWith (root))
                return true;

            if (newURL.startsWith ("juce://juce.backend"))
                return true;

           #if JUCE_DEBUG
            if (newURL.startsWith ("http://localhost:5173"))
                return true;
           #endif

            if (newURL == "about:blank")
                return true;

            return false;
        }
    };

    PrismAudioProcessor& audioProcessor;

    juce::WebComboBoxRelay keyRelay { "keySemitone" };
    juce::WebComboBoxRelay scaleRelay { "scale" };
    juce::WebSliderRelay referenceRelay { "referenceHz" };
    juce::WebSliderRelay retuneRelay { "retuneSpeedMs" };
    juce::WebSliderRelay amountRelay { "correctionAmount" };
    juce::WebSliderRelay rangeRelay { "correctionRangeSemitones" };
    juce::WebSliderRelay vibratoRelay { "vibratoPreserve" };
    juce::WebSliderRelay formantRelay { "formantFollow" };
    juce::WebSliderRelay deadZoneRelay { "quantizeDeadZoneCents" };
    juce::WebSliderRelay mixRelay { "mix" };
    juce::WebSliderRelay minF0Relay { "minF0Hz" };
    juce::WebSliderRelay dryLevelRelay { "dryLevel" };
    juce::WebToggleButtonRelay dryCorrectRelay { "dryCorrect" };
    juce::WebSliderRelay attackRelay { "attackMs" };
    juce::WebSliderRelay decayRelay { "decayMs" };
    juce::WebSliderRelay sustainRelay { "sustain" };
    juce::WebSliderRelay releaseRelay { "releaseMs" };

    SinglePageBrowser webComponent;

    juce::WebComboBoxParameterAttachment keyAttachment;
    juce::WebComboBoxParameterAttachment scaleAttachment;
    juce::WebSliderParameterAttachment referenceAttachment;
    juce::WebSliderParameterAttachment retuneAttachment;
    juce::WebSliderParameterAttachment amountAttachment;
    juce::WebSliderParameterAttachment rangeAttachment;
    juce::WebSliderParameterAttachment vibratoAttachment;
    juce::WebSliderParameterAttachment formantAttachment;
    juce::WebSliderParameterAttachment deadZoneAttachment;
    juce::WebSliderParameterAttachment mixAttachment;
    juce::WebSliderParameterAttachment minF0Attachment;
    juce::WebSliderParameterAttachment dryLevelAttachment;
    juce::WebToggleButtonParameterAttachment dryCorrectAttachment;
    juce::WebSliderParameterAttachment attackAttachment;
    juce::WebSliderParameterAttachment decayAttachment;
    juce::WebSliderParameterAttachment sustainAttachment;
    juce::WebSliderParameterAttachment releaseAttachment;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (PrismAudioProcessorEditor)
};
