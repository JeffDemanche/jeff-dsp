# dsp_add_plugin(<target>
#   PRODUCT_NAME "Display Name"
#   PLUGIN_CODE Abcd          # 4-char unique plugin code
#   [PLUGIN_MANUFACTURER_CODE Jdsp]
#   [COMPANY_NAME "Jeff DSP"]
#   [BUNDLE_ID com.jeffdsp.example]
#   [IS_SYNTH FALSE]
#   [NEEDS_MIDI_INPUT FALSE]
#   [NEEDS_MIDI_OUTPUT FALSE]
#   [IS_MIDI_EFFECT FALSE]
#   [EDITOR_WANTS_KEYBOARD_FOCUS FALSE]
#   [WEB_UI FALSE]            # enable JUCE WebBrowser + juce_gui_extra
#   [FORMATS VST3 AU Standalone]
#   [RUST_CRATE path/to/Cargo.toml]
#   [RUST_TARGET crate_name]
#   [RUST_INCLUDE path/to/include]
#   SOURCES file.cpp ...)
#
# Shared defaults for every plugin in this monorepo.
function(dsp_add_plugin target_name)
    set(options WEB_UI)
    set(oneValueArgs
        PRODUCT_NAME
        PLUGIN_CODE
        PLUGIN_MANUFACTURER_CODE
        COMPANY_NAME
        BUNDLE_ID
        IS_SYNTH
        NEEDS_MIDI_INPUT
        NEEDS_MIDI_OUTPUT
        IS_MIDI_EFFECT
        EDITOR_WANTS_KEYBOARD_FOCUS
        RUST_CRATE
        RUST_TARGET
        RUST_INCLUDE
    )
    set(multiValueArgs FORMATS SOURCES)
    cmake_parse_arguments(ARG "${options}" "${oneValueArgs}" "${multiValueArgs}" ${ARGN})

    if(NOT ARG_PRODUCT_NAME)
        message(FATAL_ERROR "dsp_add_plugin(${target_name}): PRODUCT_NAME is required")
    endif()
    if(NOT ARG_PLUGIN_CODE)
        message(FATAL_ERROR "dsp_add_plugin(${target_name}): PLUGIN_CODE is required")
    endif()
    if(NOT ARG_SOURCES)
        message(FATAL_ERROR "dsp_add_plugin(${target_name}): SOURCES is required")
    endif()

    if(NOT ARG_PLUGIN_MANUFACTURER_CODE)
        set(ARG_PLUGIN_MANUFACTURER_CODE "Jdsp")
    endif()
    if(NOT ARG_COMPANY_NAME)
        set(ARG_COMPANY_NAME "Jeff DSP")
    endif()
    if(NOT ARG_BUNDLE_ID)
        string(TOLOWER "${target_name}" _lower)
        set(ARG_BUNDLE_ID "com.jeffdsp.${_lower}")
    endif()
    if(NOT DEFINED ARG_IS_SYNTH)
        set(ARG_IS_SYNTH FALSE)
    endif()
    if(NOT DEFINED ARG_NEEDS_MIDI_INPUT)
        set(ARG_NEEDS_MIDI_INPUT FALSE)
    endif()
    if(NOT DEFINED ARG_NEEDS_MIDI_OUTPUT)
        set(ARG_NEEDS_MIDI_OUTPUT FALSE)
    endif()
    if(NOT DEFINED ARG_IS_MIDI_EFFECT)
        set(ARG_IS_MIDI_EFFECT FALSE)
    endif()
    if(NOT DEFINED ARG_EDITOR_WANTS_KEYBOARD_FOCUS)
        if(ARG_WEB_UI)
            set(ARG_EDITOR_WANTS_KEYBOARD_FOCUS TRUE)
        else()
            set(ARG_EDITOR_WANTS_KEYBOARD_FOCUS FALSE)
        endif()
    endif()
    if(NOT ARG_FORMATS)
        set(ARG_FORMATS VST3 AU Standalone)
    endif()

    # Standalone opens CoreAudio inputs; without this macOS never prompts and the
    # app will not appear under System Settings → Privacy → Microphone.
    juce_add_plugin("${target_name}"
        COMPANY_NAME "${ARG_COMPANY_NAME}"
        BUNDLE_ID "${ARG_BUNDLE_ID}"
        IS_SYNTH "${ARG_IS_SYNTH}"
        NEEDS_MIDI_INPUT "${ARG_NEEDS_MIDI_INPUT}"
        NEEDS_MIDI_OUTPUT "${ARG_NEEDS_MIDI_OUTPUT}"
        IS_MIDI_EFFECT "${ARG_IS_MIDI_EFFECT}"
        EDITOR_WANTS_KEYBOARD_FOCUS "${ARG_EDITOR_WANTS_KEYBOARD_FOCUS}"
        COPY_PLUGIN_AFTER_BUILD TRUE
        PLUGIN_MANUFACTURER_CODE "${ARG_PLUGIN_MANUFACTURER_CODE}"
        PLUGIN_CODE "${ARG_PLUGIN_CODE}"
        FORMATS ${ARG_FORMATS}
        PRODUCT_NAME "${ARG_PRODUCT_NAME}"
        MICROPHONE_PERMISSION_ENABLED TRUE
        MICROPHONE_PERMISSION_TEXT "${ARG_PRODUCT_NAME} needs microphone / audio input access to process live audio."
    )

    target_sources("${target_name}" PRIVATE ${ARG_SOURCES})

    if(ARG_WEB_UI)
        set(_web_browser 1)
        set(_gui_extra juce::juce_gui_extra)
    else()
        set(_web_browser 0)
        set(_gui_extra "")
    endif()

    target_compile_definitions("${target_name}"
        PUBLIC
            JUCE_WEB_BROWSER=${_web_browser}
            JUCE_USE_CURL=0
            JUCE_VST3_CAN_REPLACE_VST2=0
    )

    target_link_libraries("${target_name}"
        PRIVATE
            juce::juce_audio_utils
            ${_gui_extra}
            juce::juce_recommended_config_flags
            juce::juce_recommended_lto_flags
            juce::juce_recommended_warning_flags
    )

    if(ARG_RUST_CRATE)
        if(NOT ARG_RUST_TARGET)
            message(FATAL_ERROR "dsp_add_plugin(${target_name}): RUST_TARGET is required when RUST_CRATE is set")
        endif()

        corrosion_import_crate(MANIFEST_PATH "${ARG_RUST_CRATE}")
        target_link_libraries("${target_name}" PRIVATE "${ARG_RUST_TARGET}")

        if(ARG_RUST_INCLUDE)
            target_include_directories("${target_name}" PRIVATE "${ARG_RUST_INCLUDE}")
        endif()
    endif()
endfunction()
