#pragma once

#include <juce_audio_processors/juce_audio_processors.h>
#include <memory>

#include <prism_dsp.h>

class PrismAudioProcessor final : public juce::AudioProcessor
{
public:
    PrismAudioProcessor();
    ~PrismAudioProcessor() override;

    void prepareToPlay (double sampleRate, int samplesPerBlock) override;
    void releaseResources() override;
    bool isBusesLayoutSupported (const BusesLayout& layouts) const override;
    void processBlock (juce::AudioBuffer<float>&, juce::MidiBuffer&) override;

    juce::AudioProcessorEditor* createEditor() override;
    bool hasEditor() const override { return true; }

    const juce::String getName() const override { return JucePlugin_Name; }

    bool acceptsMidi() const override { return true; }
    bool producesMidi() const override { return false; }
    bool isMidiEffect() const override { return false; }
    double getTailLengthSeconds() const override { return 0.0; }

    int getNumPrograms() override { return 1; }
    int getCurrentProgram() override { return 0; }
    void setCurrentProgram (int) override {}
    const juce::String getProgramName (int) override { return {}; }
    void changeProgramName (int, const juce::String&) override {}

    void getStateInformation (juce::MemoryBlock& destData) override;
    void setStateInformation (const void* data, int sizeInBytes) override;

    juce::AudioProcessorValueTreeState& getAPVTS() noexcept { return apvts; }
    const juce::AudioProcessorValueTreeState& getAPVTS() const noexcept { return apvts; }

    /** Thread-safe snapshot for the WebView visualizer (message thread). */
    PrismDspMetering getMeteringSnapshot() const;

    static juce::AudioProcessorValueTreeState::ParameterLayout createParameterLayout();

private:
    void syncParamsToDsp();
    void updateLatency();

    struct DspDeleter
    {
        void operator() (PrismDsp* ptr) const noexcept;
    };

    juce::AudioProcessorValueTreeState apvts;
    std::unique_ptr<PrismDsp, DspDeleter> dsp;
    juce::HeapBlock<float*> channelPtrs;
    size_t channelPtrCapacity = 0;
    uint32_t lastReportedLatency = 0;

    mutable juce::SpinLock meteringLock;
    PrismDspMetering meteringSnapshot {};

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR (PrismAudioProcessor)
};
