/**
 * \file zasmrt.h
 * \brief C-compatible header for zasmrt static library (MCU / Embedded).
 */

#ifndef ZASMRT_H
#define ZASMRT_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Executes `_start` on a .vasm or .zasm bytecode array directly from static Flash memory.
 *
 * \param bytes Pointer to the static bytecode array in Flash memory.
 * \param len Length of the bytecode array in bytes.
 * \return 0 on success, negative value on error.
 */
int32_t vasm_run_bytes(const uint8_t* bytes, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* ZASMRT_H */
