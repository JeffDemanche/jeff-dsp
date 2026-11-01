dsp_add_plugin(Prism
    PRODUCT_NAME "Prism"
    PLUGIN_CODE Prsm
    BUNDLE_ID com.jeffdsp.prism
    FORMATS VST3 AU Standalone
    RUST_CRATE "${CMAKE_CURRENT_SOURCE_DIR}/rust/Cargo.toml"
    RUST_TARGET prism_dsp
    RUST_INCLUDE "${CMAKE_CURRENT_SOURCE_DIR}/rust/include"
    SOURCES
        Source/PluginProcessor.cpp
        Source/PluginProcessor.h
        Source/PluginEditor.cpp
        Source/PluginEditor.h
)
