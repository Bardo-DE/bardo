//! Random bytes from the operating system's source (the `getrandom` crate), for keys, nonces,
//! tokens and ids.
//!
//! Every function returns an error when the system gives no bytes. There is no fallback to the
//! clock or the process id: a secret made from those is guessable, and an id made from them hides
//! a broken system. The caller says what a failure means for it.

/// The system gave no random bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the system gave no random bytes")]
pub struct RandomError(#[source] getrandom::Error);

/// Fills `buffer` with random bytes.
///
/// # Errors
/// [`RandomError`] when the system gives no random bytes.
pub fn fill(buffer: &mut [u8]) -> Result<(), RandomError> {
    getrandom::fill(buffer).map_err(RandomError)
}

/// `N` random bytes.
///
/// # Errors
/// [`RandomError`] when the system gives no random bytes.
pub fn bytes<const N: usize>() -> Result<[u8; N], RandomError> {
    let mut bytes = [0_u8; N];
    fill(&mut bytes)?;
    Ok(bytes)
}

/// `N` random bytes as lower-case hex: `2 * N` characters, `0-9` and `a-f`.
///
/// # Errors
/// [`RandomError`] when the system gives no random bytes.
pub fn hex<const N: usize>() -> Result<String, RandomError> {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    Ok(bytes::<N>()?
        .iter()
        .flat_map(|b| [DIGITS[usize::from(b >> 4)], DIGITS[usize::from(b & 15)]])
        .map(char::from)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_is_twice_as_long_as_the_bytes_and_lower_case_hex() {
        let text = hex::<16>().expect("random");
        assert_eq!(text.len(), 32);
        assert!(text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
    }

    #[test]
    fn two_draws_differ() {
        assert_ne!(
            bytes::<32>().expect("first"),
            bytes::<32>().expect("second")
        );
    }

    #[test]
    fn fill_fills_the_whole_buffer() {
        // 64 zero bytes in a row from a working source has a chance of 2^-512.
        let mut buffer = [0_u8; 64];
        fill(&mut buffer).expect("random");
        assert!(buffer.iter().any(|b| *b != 0));
    }

    #[test]
    fn an_error_keeps_its_cause() {
        let error = RandomError(getrandom::Error::UNSUPPORTED);
        assert!(std::error::Error::source(&error).is_some());
    }
}
