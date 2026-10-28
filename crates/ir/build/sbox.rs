//! Static S-box permutation generator for Wasmi IR opcodes.
//!
//! Replaces the default sequential opcode numbering with a pseudo-random
//! bijective permutation so that the bytecode opcodes are statically scrambled,
//! preventing casual inspection or reverse-engineering of the IR.

/// Generates a static pseudo-random bijective S-box permutation of size `len`.
///
/// Uses a deterministic SplitMix64 PRNG to perform a Fisher-Yates shuffle.
/// Every number in `0..len` appears exactly once:
/// - 0 gaps, 0 duplicates.
/// - Strictly bijective mapping.
pub fn generate_sbox(len: usize) -> Vec<u16> {
    let mut table: Vec<u16> = (0..len as u16).collect();
    // Deterministic constant seed
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;

    let mut next_rand = || -> u64 {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };

    for i in (1..len).rev() {
        let j = (next_rand() as usize) % (i + 1);
        table.swap(i, j);
    }

    table
}
