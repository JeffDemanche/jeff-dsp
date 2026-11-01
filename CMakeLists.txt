cmake_minimum_required(VERSION 3.25)

project(jeff_dsp
    VERSION 0.1.0
    DESCRIPTION "JUCE + Rust DSP plugin monorepo"
    LANGUAGES C CXX
)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Semicolon-separated plugin directory names under plugins/, or empty for all.
set(DSP_PLUGINS "" CACHE STRING "Plugins to build (e.g. prism). Empty = all.")

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")
include(Dependencies)
include(AddPlugin)

# Discover and register shared custom JUCE modules (if any).
file(GLOB _dsp_module_dirs LIST_DIRECTORIES true "${CMAKE_CURRENT_SOURCE_DIR}/modules/*")
foreach(_mod_dir IN LISTS _dsp_module_dirs)
    if(IS_DIRECTORY "${_mod_dir}" AND EXISTS "${_mod_dir}/juce_module_info")
        get_filename_component(_mod_name "${_mod_dir}" NAME)
        message(STATUS "Adding shared JUCE module: ${_mod_name}")
        juce_add_module("${_mod_dir}")
    endif()
endforeach()

# Select plugins.
set(_plugin_root "${CMAKE_CURRENT_SOURCE_DIR}/plugins")
if(DSP_PLUGINS STREQUAL "")
    file(GLOB _plugin_dirs LIST_DIRECTORIES true "${_plugin_root}/*")
    set(_selected_plugins "")
    foreach(_dir IN LISTS _plugin_dirs)
        if(IS_DIRECTORY "${_dir}" AND EXISTS "${_dir}/CMakeLists.txt")
            get_filename_component(_name "${_dir}" NAME)
            list(APPEND _selected_plugins "${_name}")
        endif()
    endforeach()
else()
    set(_selected_plugins ${DSP_PLUGINS})
endif()

list(SORT _selected_plugins)
list(REMOVE_DUPLICATES _selected_plugins)

if(_selected_plugins STREQUAL "")
    message(WARNING "No plugins selected to build under ${_plugin_root}")
endif()

foreach(_plugin IN LISTS _selected_plugins)
    set(_plugin_dir "${_plugin_root}/${_plugin}")
    if(NOT EXISTS "${_plugin_dir}/CMakeLists.txt")
        message(FATAL_ERROR "Plugin '${_plugin}' not found at ${_plugin_dir}")
    endif()
    message(STATUS "Adding plugin: ${_plugin}")
    add_subdirectory("${_plugin_dir}" "plugins/${_plugin}")
endforeach()
