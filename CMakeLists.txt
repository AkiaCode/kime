cmake_minimum_required(VERSION 3.1.0)

project(kime VERSION 0.9.1)

option(ENABLE_GTK2 "Enable GTK2 immodule" OFF)
option(ENABLE_GTK3 "Enable GTK3 immodule" OFF)
option(ENABLE_GTK4 "Enable GTK4 immodule" OFF)
option(ENABLE_QT5 "Enable Qt5 immodule" OFF)
option(ENABLE_QT6 "Enable Qt6 immodule" OFF)

set(CMAKE_SKIP_RPATH TRUE)
set(CMAKE_C_FLAGS_DEBUG "${CMAKE_C_FLAGS_DEBUG} -DDEBUG")
set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -DDEBUG")
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
set(KIME_INCLUDE ${CMAKE_CURRENT_SOURCE_DIR}/engine/cffi)
set(OpenGL_GL_PREFERENCE GLVND)

include(GNUInstallDirs)

if(${ENABLE_GTK2})
    add_subdirectory(gtk2)
endif()
if(${ENABLE_GTK3})
    add_subdirectory(gtk3)
endif()
if(${ENABLE_GTK4})
    add_subdirectory(gtk4)
endif()

if(${ENABLE_QT5})
    add_subdirectory(qt5)
endif()

if(${ENABLE_QT6})
    add_subdirectory(qt6)
endif()
