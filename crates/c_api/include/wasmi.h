#ifndef WASMI_H
#define WASMI_H

#include <wasm.h>
#include <wasmi/config.h>
#include <wasmi/engine.h>
#include <wasmi/error.h>
#include <wasmi/store.h>

/**
 * \brief Wasmi version string.
 */
#define WASMI_VERSION "0.35.0"
/**
 * \brief Wasmi major version number.
 */
#define WASMI_VERSION_MAJOR 0
/**
 * \brief Wasmi minor version number.
 */
#define WASMI_VERSION_MINOR 35
/**
 * \brief Wasmi patch version number.
 */
#define WASMI_VERSION_PATCH 0

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Creates a new Wasm module directly from static Flash memory.
 *
 * Precompiled Wasmi IR bytecode will be executed directly from Flash (XIP)
 * without allocating heap memory for the bytecode.
 *
 * \param store The store to associate with the module.
 * \param bytes Pointer to the static byte array in Flash memory.
 * \param len Length of the static byte array in bytes.
 * \return A new #wasm_module_t or NULL on failure.
 */
wasm_module_t* wasmi_module_new_static(
    wasm_store_t* store,
    const uint8_t* bytes,
    size_t len
);

#ifdef __cplusplus
}
#endif

#endif // WASMI_H
