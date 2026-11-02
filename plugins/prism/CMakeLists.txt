dsp_add_plugin(Prism
    PRODUCT_NAME "Prism"
    PLUGIN_CODE Prsm
    BUNDLE_ID com.jeffdsp.prism
    FORMATS VST3 AU Standalone
    WEB_UI
    RUST_CRATE "${CMAKE_CURRENT_SOURCE_DIR}/rust/Cargo.toml"
    RUST_TARGET prism_dsp
    RUST_INCLUDE "${CMAKE_CURRENT_SOURCE_DIR}/rust/include"
    SOURCES
        Source/PluginProcessor.cpp
        Source/PluginProcessor.h
        Source/PluginEditor.cpp
        Source/PluginEditor.h
)

# Pack WebView UI assets (prefer ui/dist; fall back to placeholder HTML).
set(PRISM_UI_DIST "${CMAKE_CURRENT_SOURCE_DIR}/ui/dist")
set(PRISM_UI_FALLBACK "${CMAKE_CURRENT_SOURCE_DIR}/ui_fallback/index.html")

file(GLOB PRISM_UI_ASSETS CONFIGURE_DEPENDS
    "${PRISM_UI_DIST}/*"
    "${PRISM_UI_DIST}/*/*"
)

if(PRISM_UI_ASSETS)
    set(PRISM_UI_SOURCES ${PRISM_UI_ASSETS})
    message(STATUS "Prism UI: packing assets from ui/dist")
else()
    set(PRISM_UI_SOURCES "${PRISM_UI_FALLBACK}")
    message(STATUS "Prism UI: ui/dist missing — packing ui_fallback/index.html (run npm run build in plugins/prism/ui)")
endif()

juce_add_binary_data(PrismUiData
    HEADER_NAME PrismUiData.h
    NAMESPACE PrismUiData
    SOURCES ${PRISM_UI_SOURCES}
)

target_link_libraries(Prism PRIVATE PrismUiData)

target_compile_definitions(Prism
    PUBLIC
        $<$<CONFIG:Debug>:PRISM_UI_DEV_SERVER_DEFAULT=0>
)
