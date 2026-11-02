#include "PluginEditor.h"

#include <PrismUiData.h>

#include <unordered_map>

namespace
{
const char* mimeForExtension (const juce::String& extension)
{
    static const std::unordered_map<juce::String, const char*> mimeMap {
        { "htm",  "text/html" },
        { "html", "text/html" },
        { "txt",  "text/plain" },
        { "jpg",  "image/jpeg" },
        { "jpeg", "image/jpeg" },
        { "svg",  "image/svg+xml" },
        { "ico",  "image/vnd.microsoft.icon" },
        { "json", "application/json" },
        { "png",  "image/png" },
        { "css",  "text/css" },
        { "js",   "text/javascript" },
        { "mjs",  "text/javascript" },
        { "woff", "font/woff" },
        { "woff2","font/woff2" },
        { "ttf",  "font/ttf" },
        { "map",  "application/json" },
    };

    if (const auto it = mimeMap.find (extension.toLowerCase()); it != mimeMap.end())
        return it->second;

    return "application/octet-stream";
}

juce::String resourcePathFromUrl (const juce::String& url)
{
    if (url == "/" || url.isEmpty())
        return "index.html";

    auto path = url;
    while (path.startsWith ("/"))
        path = path.fromFirstOccurrenceOf ("/", false, false);

    path = path.upToFirstOccurrenceOf ("?", false, false);
    path = path.upToFirstOccurrenceOf ("#", false, false);

    while (path.startsWith ("./"))
        path = path.substring (2);

    return path.isEmpty() ? juce::String ("index.html") : path;
}

bool useViteDevServer()
{
   #if JUCE_DEBUG
    if (juce::SystemStats::getEnvironmentVariable ("PRISM_UI_DEV_SERVER", {}) == "1")
        return true;
   #endif
    return false;
}

constexpr auto viteDevServerAddress = "http://localhost:5173";
} // namespace

PrismAudioProcessorEditor::PrismAudioProcessorEditor (PrismAudioProcessor& p)
    : AudioProcessorEditor (&p),
      audioProcessor (p),
      webComponent (juce::WebBrowserComponent::Options{}
                      #if JUCE_WINDOWS
                       .withBackend (juce::WebBrowserComponent::Options::Backend::webview2)
                       .withWinWebView2Options (juce::WebBrowserComponent::Options::WinWebView2{}
                                                    .withUserDataFolder (juce::File::getSpecialLocation (
                                                        juce::File::SpecialLocationType::tempDirectory)))
                      #endif
                       .withNativeIntegrationEnabled()
                       .withKeepPageLoadedWhenBrowserIsHidden()
                       .withOptionsFrom (keyRelay)
                       .withOptionsFrom (scaleRelay)
                       .withOptionsFrom (referenceRelay)
                       .withOptionsFrom (retuneRelay)
                       .withOptionsFrom (amountRelay)
                       .withOptionsFrom (rangeRelay)
                       .withOptionsFrom (vibratoRelay)
                       .withOptionsFrom (formantRelay)
                       .withOptionsFrom (deadZoneRelay)
                       .withOptionsFrom (mixRelay)
                       .withOptionsFrom (minF0Relay)
                       .withOptionsFrom (dryLevelRelay)
                       .withOptionsFrom (dryCorrectRelay)
                       .withOptionsFrom (attackRelay)
                       .withOptionsFrom (decayRelay)
                       .withOptionsFrom (sustainRelay)
                       .withOptionsFrom (releaseRelay)
                       .withResourceProvider ([this] (const auto& url)
                                              {
                                                  return getResource (url);
                                              },
                                              juce::URL { viteDevServerAddress }.getOrigin())),
      keyAttachment (*audioProcessor.getAPVTS().getParameter ("keySemitone"), keyRelay, nullptr),
      scaleAttachment (*audioProcessor.getAPVTS().getParameter ("scale"), scaleRelay, nullptr),
      referenceAttachment (*audioProcessor.getAPVTS().getParameter ("referenceHz"), referenceRelay, nullptr),
      retuneAttachment (*audioProcessor.getAPVTS().getParameter ("retuneSpeedMs"), retuneRelay, nullptr),
      amountAttachment (*audioProcessor.getAPVTS().getParameter ("correctionAmount"), amountRelay, nullptr),
      rangeAttachment (*audioProcessor.getAPVTS().getParameter ("correctionRangeSemitones"), rangeRelay, nullptr),
      vibratoAttachment (*audioProcessor.getAPVTS().getParameter ("vibratoPreserve"), vibratoRelay, nullptr),
      formantAttachment (*audioProcessor.getAPVTS().getParameter ("formantFollow"), formantRelay, nullptr),
      deadZoneAttachment (*audioProcessor.getAPVTS().getParameter ("quantizeDeadZoneCents"), deadZoneRelay, nullptr),
      mixAttachment (*audioProcessor.getAPVTS().getParameter ("mix"), mixRelay, nullptr),
      minF0Attachment (*audioProcessor.getAPVTS().getParameter ("minF0Hz"), minF0Relay, nullptr),
      dryLevelAttachment (*audioProcessor.getAPVTS().getParameter ("dryLevel"), dryLevelRelay, nullptr),
      dryCorrectAttachment (*audioProcessor.getAPVTS().getParameter ("dryCorrect"), dryCorrectRelay, nullptr),
      attackAttachment (*audioProcessor.getAPVTS().getParameter ("attackMs"), attackRelay, nullptr),
      decayAttachment (*audioProcessor.getAPVTS().getParameter ("decayMs"), decayRelay, nullptr),
      sustainAttachment (*audioProcessor.getAPVTS().getParameter ("sustain"), sustainRelay, nullptr),
      releaseAttachment (*audioProcessor.getAPVTS().getParameter ("releaseMs"), releaseRelay, nullptr)
{
    webComponent.onPageFinished = [this]
    {
        webComponent.setBounds (getLocalBounds());
    };

    addAndMakeVisible (webComponent);
    setSize (720, 640);
    setResizable (true, true);
    setResizeLimits (480, 420, 1400, 1100);
    webComponent.goToURL (resolveInitialUrl());
    startTimerHz (30);
}

void PrismAudioProcessorEditor::timerCallback()
{
    const auto metering = audioProcessor.getMeteringSnapshot();
    webComponent.emitEventIfBrowserIsVisible ("prismMetering", meteringToVar (metering));
}

juce::var PrismAudioProcessorEditor::meteringToVar (const PrismDspMetering& m) const
{
    auto* obj = new juce::DynamicObject();
    obj->setProperty ("detectedHz", m.detected_hz);
    obj->setProperty ("dryTargetHz", m.dry_target_hz);
    obj->setProperty ("numVoices", static_cast<int> (m.num_voices));

    juce::Array<juce::var> voices;
    const auto n = juce::jmin (static_cast<int> (m.num_voices), static_cast<int> (PRISM_DSP_MAX_VOICES));
    for (int i = 0; i < n; ++i)
    {
        auto* voice = new juce::DynamicObject();
        voice->setProperty ("midi", m.voice_midi[i]);
        voice->setProperty ("targetHz", m.voice_target_hz[i]);
        voices.add (juce::var (voice));
    }
    obj->setProperty ("voices", voices);
    return juce::var (obj);
}

juce::String PrismAudioProcessorEditor::resolveInitialUrl() const
{
    if (useViteDevServer())
        return viteDevServerAddress;

    return juce::WebBrowserComponent::getResourceProviderRoot();
}

std::optional<juce::WebBrowserComponent::Resource> PrismAudioProcessorEditor::getResource (const juce::String& url) const
{
    const auto urlToFind = resourcePathFromUrl (url);

    for (int i = 0; i < PrismUiData::namedResourceListSize; ++i)
    {
        const auto originalName = juce::String (PrismUiData::originalFilenames[i]);

        if (urlToFind.equalsIgnoreCase (originalName)
            || urlToFind.endsWithIgnoreCase ("/" + originalName))
        {
            int dataSize = 0;
            const auto* data = PrismUiData::getNamedResource (PrismUiData::namedResourceList[i], dataSize);
            if (data == nullptr || dataSize <= 0)
                continue;

            const auto mime = mimeForExtension (juce::File (originalName).getFileExtension().substring (1));
            return juce::WebBrowserComponent::Resource {
                std::vector<std::byte> (reinterpret_cast<const std::byte*> (data),
                                        reinterpret_cast<const std::byte*> (data) + dataSize),
                mime
            };
        }
    }

    return std::nullopt;
}

void PrismAudioProcessorEditor::paint (juce::Graphics& g)
{
    g.fillAll (juce::Colour (0xff12141a));
}

void PrismAudioProcessorEditor::resized()
{
    webComponent.setBounds (getLocalBounds());
}
