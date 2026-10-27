# Núcleo de quadro do processo hospedeiro. Não é toolchain do Android NDK:
# o CMake daqui gera a biblioteca do Mac, do Windows ou do Linux.
# O binário Rust liga o arquivo estático (um executável só no pacote).
# A biblioteca compartilhada é o mesmo código, para o esqueleto existir
# como .dylib / .so / .dll. Ver docs/arquitetura/ADR-001-nucleo-nativo-e-android-studio.md.
cmake_minimum_required(VERSION 3.20)
project(multidroid_frame C)

set(MD_SOURCES src/frame.c src/pump.c)

add_library(multidroid_frame SHARED ${MD_SOURCES})
add_library(multidroid_frame_static STATIC ${MD_SOURCES})

foreach(tgt multidroid_frame multidroid_frame_static)
  target_include_directories(${tgt} PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
  target_compile_features(${tgt} PRIVATE c_std_11)
  if(APPLE)
    target_compile_definitions(${tgt} PRIVATE MD_BUILD_FOR_APPLE=1)
  elseif(WIN32)
    target_compile_definitions(${tgt} PRIVATE MD_BUILD_FOR_WINDOWS=1)
  else()
    target_compile_definitions(${tgt} PRIVATE MD_BUILD_FOR_LINUX=1)
  endif()
  if(MSVC)
    target_compile_options(${tgt} PRIVATE /W4 /FS)
  else()
    target_compile_options(${tgt} PRIVATE -Wall -Wextra -Werror)
  endif()
endforeach()

target_compile_definitions(multidroid_frame PRIVATE MD_FRAME_BUILD_SHARED)
set_target_properties(multidroid_frame PROPERTIES
  C_VISIBILITY_PRESET hidden
  POSITION_INDEPENDENT_CODE ON
  OUTPUT_NAME multidroid_frame
  VERSION 0.3.0
  SOVERSION 0)
set_target_properties(multidroid_frame_static PROPERTIES
  POSITION_INDEPENDENT_CODE ON
  OUTPUT_NAME multidroid_frame_static)

if(NOT WIN32)
  find_package(Threads REQUIRED)
  target_link_libraries(multidroid_frame PUBLIC Threads::Threads)
  target_link_libraries(multidroid_frame_static PUBLIC Threads::Threads)
endif()

install(TARGETS multidroid_frame multidroid_frame_static
  ARCHIVE DESTINATION lib
  LIBRARY DESTINATION lib
  RUNTIME DESTINATION bin)
install(FILES include/md_frame.h DESTINATION include)
