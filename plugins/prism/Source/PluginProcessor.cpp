#include "PluginProcessor.h"
#include "PluginEditor.h"

namespace
{
constexpr auto paramKeySemitone = "keySemitone";
constexpr auto paramScale = "scale";
constexpr auto paramReferenceHz = "referenceHz";
constexpr auto paramRetuneSpeedMs = "retuneSpeedMs";
constexpr auto paramCorrectionAmount = "correctionAmount";
constexpr auto paramCorrectionRange = "correctionRangeSemitones";
constexpr auto paramVibratoPreserve = "vibratoPreserve";
constexpr auto paramFormantFollow = "formantFollow";
constexpr auto paramDeadZone = "quantizeDeadZoneCents";
constexpr auto paramMix = "mix";
constexpr auto paramMinF0Hz = "minF0Hz";
constexpr auto paramDryLevel = "dryLevel";
constexpr auto paramDryCorrect = "dryCorrect";
} // namespace

void PrismAudioProcessor::DspDeleter::operator() (PrismDsp* ptr) const noexcept
{
    prism_dsp_destroy (ptr);
}

juce::AudioProcessorValueTreeState::ParameterLayout PrismAudioProcessor::createParameterLayout()
{
    std::vector<std::unique_ptr<juce::RangedAudioParameter>> params;

    params.push_back (std::make_unique<juce::AudioParameterChoice> (
        juce::ParameterID { paramKeySemitone, 1 },
        "Key",
        juce::StringArray { "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B" },
        0));

    params.push_back (std::make_unique<juce::AudioParameterChoice> (
        juce::ParameterID { paramScale, 1 },
        "Scale",
        juce::StringArray { "Chromatic", "Major", "Natural Minor" },
        1));

    params.push_back (std::make_unique<juce::AudioParameterFloat> (
        juce::ParameterID { paramReferenceHz, 1 },
        "Reference",
        juce::NormalisableRange<float> (400.0f, 480.0f, 0.1f),
        440.0f,
        juce::AudioParameterFloatAttributes().withLabel ("Hz")));

    params.push_back (std::make_unique<juce::AudioParameterFloat> (
        juce::ParameterID { paramRetuneSpeedMs, 1 },
        "Retune Speed",
        juce::NormalisableRange<float> (0.0f, 500.0f, 0.1f, 0.4f),
        20.0f,
        juce::AudioParameterFloatAttributes().withLabel ("ms")));

    params.push_back (std::make_unique<juce::AudioParameterFloat> (
        juce::ParameterID { paramCorrectionAmount, 1 },
        "Amount",
        juce::NormalisableRange<float> (0.0f, 1.0f, 0.001f),
        1.0f));

    params.push_back (std::make_unique<juce::AudioParameterFloat> (
        juce::ParameterID { paramCorrectionRange, 1 },
        "Range",
        juce::NormalisableRange<float> (0.0f, 12.0f, 0.01f),
        2.0f,
        juce::AudioParameterFloatAttributes().withLabel ("st")));

    params.push_back (std::make_unique<juce::AudioParameterFloat> (
        juce::ParameterID { paramVibratoPreserve, 1 },
        "Vibrato",
        juce::NormalisableRange<float> (0.0f, 1.0f, 0.001f),
        0.7f));

    params.push_back (std::make_unique<juce::AudioParameterFloat> (
        juce::ParameterID { paramFormantFollow, 1 },
        "Formant Follow",
        juce::NormalisableRange<float> (0.0f, 1.0f, 0.001f),
        0.0f));

    params.push_back (std::make_unique<juce::AudioParameterFloat> (
        juce::ParameterID { paramDeadZone, 1 },
        "Dead Zone",
        juce::NormalisableRange<float> (0.0f, 50.0f, 0.1f),
        8.0f,
        juce::AudioParameterFloatAttributes().withLabel ("cents")));

    params.push_back (std::make_unique<juce::AudioParameterFloat> (
        juce::ParameterID { paramMix, 1 },
        "Voices Level",
        juce::NormalisableRange<float> (0.0f, 1.0f, 0.001f),
        1.0f));

    params.push_back (std::make_unique<juce::AudioParameterFloat> (
        juce::ParameterID { paramMinF0Hz, 1 },
        "F0 Floor",
        juce::NormalisableRange<float> (50.0f, 200.0f, 0.1f),
        50.0f,
        juce::AudioParameterFloatAttributes().withLabel ("Hz")));

    params.push_back (std::make_unique<juce::AudioParameterFloat> (
        juce::ParameterID { paramDryLevel, 1 },
        "Dry Level",
        juce::NormalisableRange<float> (0.0f, 1.0f, 0.001f),
        1.0f));

    params.push_back (std::make_unique<juce::AudioParameterBool> (
        juce::ParameterID { paramDryCorrect, 1 },
        "Dry Correct",
        true));

    return { params.begin(), params.end() };
}

PrismAudioProcessor::PrismAudioProcessor()
    : AudioProcessor (BusesProperties()
                          .withInput  ("Input",  juce::AudioChannelSet::stereo(), true)
                          .withOutput ("Output", juce::AudioChannelSet::stereo(), true)),
      apvts (*this, nullptr, "PARAMS", createParameterLayout()),
      dsp (prism_dsp_create())
{
    jassert (dsp != nullptr);
    syncParamsToDsp();
    updateLatency();
}

PrismAudioProcessor::~PrismAudioProcessor() = default;

PrismDspMetering PrismAudioProcessor::getMeteringSnapshot() const
{
    const juce::SpinLock::ScopedLockType lock (meteringLock);
    return meteringSnapshot;
}

void PrismAudioProcessor::syncParamsToDsp()
{
    // APVTS getRawParameterValue() is already denormalised (Hz, ms, …).
    // Do NOT convertFrom0to1 on it — that clamps e.g. minF0=80 → range max 200.
    auto raw = [this] (const char* id) -> float
    {
        auto* v = apvts.getRawParameterValue (id);
        jassert (v != nullptr);
        return v->load();
    };

    auto choiceIndex = [this] (const char* id) -> int
    {
        auto* param = apvts.getParameter (id);
        jassert (param != nullptr);
        // Choice params expose normalised 0–1 via getValue(); map to index.
        if (auto* choice = dynamic_cast<juce::AudioParameterChoice*> (param))
            return choice->getIndex();
        return juce::roundToInt (param->convertFrom0to1 (param->getValue()));
    };

    PrismDspParams p {};
    p.key_semitone = static_cast<uint8_t> (choiceIndex (paramKeySemitone));
    p.scale = static_cast<uint8_t> (choiceIndex (paramScale));
    p.reference_hz = raw (paramReferenceHz);
    p.retune_speed_ms = raw (paramRetuneSpeedMs);
    p.correction_amount = raw (paramCorrectionAmount);
    p.correction_range_semitones = raw (paramCorrectionRange);
    p.vibrato_preserve = raw (paramVibratoPreserve);
    p.formant_follow = raw (paramFormantFollow);
    p.quantize_dead_zone_cents = raw (paramDeadZone);
    p.mix = raw (paramMix);
    p.min_f0_hz = raw (paramMinF0Hz);
    p.dry_level = raw (paramDryLevel);
    p.dry_correct = raw (paramDryCorrect) >= 0.5f ? 1 : 0;
    prism_dsp_set_params (dsp.get(), &p);
}

void PrismAudioProcessor::updateLatency()
{
    const auto latency = prism_dsp_reported_latency_samples (dsp.get());
    if (latency != lastReportedLatency)
    {
        lastReportedLatency = latency;
        setLatencySamples (static_cast<int> (latency));
    }
}

void PrismAudioProcessor::prepareToPlay (double sampleRate, int samplesPerBlock)
{
    channelPtrCapacity = static_cast<size_t> (juce::jmax (1, getTotalNumOutputChannels()));
    channelPtrs.calloc (channelPtrCapacity);
    prism_dsp_prepare (dsp.get(), sampleRate, static_cast<uint32_t> (samplesPerBlock));
    syncParamsToDsp();
    prism_dsp_reset (dsp.get());
    updateLatency();
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
    juce::ScopedNoDenormals noDenormals;

    syncParamsToDsp();
    updateLatency();

    for (const auto metadata : midi)
    {
        const auto msg = metadata.getMessage();
        if (msg.isNoteOn())
            prism_dsp_note_on (dsp.get(),
                               static_cast<uint8_t> (msg.getNoteNumber()),
                               msg.getFloatVelocity());
        else if (msg.isNoteOff())
            prism_dsp_note_off (dsp.get(), static_cast<uint8_t> (msg.getNoteNumber()));
        else if (msg.isAllNotesOff() || msg.isAllSoundOff())
        {
            for (uint8_t n = 0; n < 128; ++n)
                prism_dsp_note_off (dsp.get(), n);
        }
    }

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

    PrismDspMetering metering {};
    prism_dsp_get_metering (dsp.get(), &metering);
    {
        const juce::SpinLock::ScopedLockType lock (meteringLock);
        meteringSnapshot = metering;
    }
}

void PrismAudioProcessor::getStateInformation (juce::MemoryBlock& destData)
{
    if (auto xml = apvts.copyState().createXml())
        copyXmlToBinary (*xml, destData);
}

void PrismAudioProcessor::setStateInformation (const void* data, int sizeInBytes)
{
    if (auto xml = getXmlFromBinary (data, sizeInBytes))
        if (xml->hasTagName (apvts.state.getType()))
            apvts.replaceState (juce::ValueTree::fromXml (*xml));
}

juce::AudioProcessorEditor* PrismAudioProcessor::createEditor()
{
    return new PrismAudioProcessorEditor (*this);
}

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()
{
    return new PrismAudioProcessor();
}
