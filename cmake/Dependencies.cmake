include(FetchContent)

# Pin JUCE 8.x for a stable plugin API surface.
FetchContent_Declare(
    JUCE
    GIT_REPOSITORY https://github.com/juce-framework/JUCE.git
    GIT_TAG        8.0.9
    GIT_SHALLOW    TRUE
)

# Corrosion builds Rust crates as first-class CMake targets.
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG        v0.5.2
    GIT_SHALLOW    TRUE
)

FetchContent_MakeAvailable(JUCE Corrosion)
