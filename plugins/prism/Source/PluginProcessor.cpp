#include "PluginProcessor.h"
#include "PluginEditor.h"

#include <prism_dsp.h>

void PrismAudioProcessor::DspDeleter::operator() (PrismDsp* ptr) const noexcept
{
    prism_dsp_destroy (ptr);
}

PrismAudioProcessor::PrismAudioProcessor()
    : AudioProcessor (BusesProperties()
                          .withInput  ("Input",  juce::AudioChannelSet::stereo(), true)
                          .withOutput ("Output", juce::AudioChannelSet::stereo(), true)),
      dsp (prism_dsp_create())
{
    jassert (dsp != nullptr);
    prism_dsp_set_gain (dsp.get(), 1.0f);
}

PrismAudioProcessor::~PrismAudioProcessor() = default;

void PrismAudioProcessor::prepareToPlay (double sampleRate, int samplesPerBlock)
{
    channelPtrCapacity = static_cast<size_t> (juce::jmax (1, getTotalNumOutputChannels()));
    channelPtrs.calloc (channelPtrCapacity);
    prism_dsp_prepare (dsp.get(), sampleRate, static_cast<uint32_t> (samplesPerBlock));
    prism_dsp_reset (dsp.get());
}

void PrismAudioProcessor::releaseResources()
{
    prism_dsp_reset (dsp.get());
}

bool PrismAudioProcessor::isBusesLayoutSupported (const BusesLayout& layouts) const
{
    if (layouts.getMainOutputChannelSet() != juce::AudioChannelSet::mono()
        && layouts.getMainOutputChannelSet() != juce::AudioChannelSet::stereo())
        return false;

    if (layouts.getMainOutputChannelSet() != layouts.getMainInputChannelSet())
        return false;

    return true;
}

void PrismAudioProcessor::processBlock (juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midi)
{
    juce::ignoreUnused (midi);
    juce::ScopedNoDenormals noDenormals;

    const auto totalNumInputChannels  = getTotalNumInputChannels();
    const auto totalNumOutputChannels = getTotalNumOutputChannels();

    for (auto i = totalNumInputChannels; i < totalNumOutputChannels; ++i)
        buffer.clear (i, 0, buffer.getNumSamples());

    const auto numChannels = static_cast<uint32_t> (buffer.getNumChannels());
    const auto numSamples  = static_cast<uint32_t> (buffer.getNumSamples());

    if (channelPtrCapacity < numChannels)
    {
        channelPtrCapacity = numChannels;
        channelPtrs.calloc (channelPtrCapacity);
    }

    for (uint32_t ch = 0; ch < numChannels; ++ch)
        channelPtrs[ch] = buffer.getWritePointer (static_cast<int> (ch));

    prism_dsp_process (dsp.get(), channelPtrs.getData(), numChannels, numSamples);
}

void PrismAudioProcessor::getStateInformation (juce::MemoryBlock& destData)
{
    juce::ignoreUnused (destData);
}

void PrismAudioProcessor::setStateInformation (const void* data, int sizeInBytes)
{
    juce::ignoreUnused (data, sizeInBytes);
}

juce::AudioProcessorEditor* PrismAudioProcessor::createEditor()
{
    return new PrismAudioProcessorEditor (*this);
}

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()
{
    return new PrismAudioProcessor();
}
