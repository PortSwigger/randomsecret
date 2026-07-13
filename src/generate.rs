// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: The randomsecret contributors

use rand::{CryptoRng, Rng, RngExt};

/// The base62 alphabet in the ordering used by zqlu
/// (<https://github.com/nresare/zqlu>): digits, upper case, lower case.
const ALPHABET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Number of alphabet entries that are digits, and thus not allowed as the
/// first character of a generated value.
const DIGITS: usize = 10;

/// Default number of characters in a generated value. The first character is
/// drawn from 52 letters (log2(52) ~ 5.70 bits) and the remaining ones from
/// the full 62 character alphabet (log2(62) ~ 5.954 bits), so 44 is the
/// smallest length carrying at least 256 bits of entropy. We round that up to
/// the next multiple of 3 so that Kubernetes' base64 encoding of the value
/// needs no `=` padding.
pub const DEFAULT_LENGTH: usize = 45;

/// Generate a random base62 string of `length` characters whose first
/// character is never a digit.
pub fn generate<R: Rng + CryptoRng>(length: usize, rng: &mut R) -> String {
    (0..length)
        .map(|i| {
            let start = if i == 0 { DIGITS } else { 0 };
            ALPHABET[rng.random_range(start..ALPHABET.len())] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn generates_requested_length() {
        let mut rng = StdRng::seed_from_u64(0);
        for length in [1, 40, DEFAULT_LENGTH, 100] {
            assert_eq!(generate(length, &mut rng).len(), length);
        }
    }

    #[test]
    fn first_character_is_never_a_digit() {
        let mut rng = StdRng::seed_from_u64(1);
        for _ in 0..1000 {
            let value = generate(1, &mut rng);
            assert!(!value.chars().next().unwrap().is_ascii_digit(), "{value}");
        }
    }

    #[test]
    fn only_uses_alphabet_characters() {
        let mut rng = StdRng::seed_from_u64(2);
        let value = generate(10_000, &mut rng);
        assert!(value.bytes().all(|b| ALPHABET.contains(&b)));
        // with 10k samples every alphabet character should show up
        for b in ALPHABET {
            assert!(value.bytes().any(|v| v == *b), "missing {}", *b as char);
        }
    }

    #[test]
    fn values_are_distinct() {
        let mut rng = StdRng::seed_from_u64(3);
        assert_ne!(
            generate(DEFAULT_LENGTH, &mut rng),
            generate(DEFAULT_LENGTH, &mut rng)
        );
    }
}
