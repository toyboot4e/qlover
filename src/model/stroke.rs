//! Port of [`plover_stroke`](https://github.com/openstenoproject/plover_stroke) and the stroke
//! module.
//!
//! # Notations
//!
//! In this module, we denote the number of keys as _n_.

#[cfg(test)]
mod test;

use thiserror::Error;

use std::{cmp::Ordering, fmt, ops};

/// `None` | `Left` | `Right`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeySide {
    None,
    Left,
    Right,
}

/// One of `<letter>`, `<letter>-` or `-<letter>`.
///
/// # Examples
///
/// ```
/// use qlover::model::stroke::{KeySide, LetterWithSide};
/// assert_eq!(
///     LetterWithSide::parse("a-"),
///     Some(LetterWithSide {
///         letter: 'a',
///         side: KeySide::Left
///     })
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LetterWithSide {
    pub letter: char,
    pub side: KeySide,
}

impl fmt::Display for LetterWithSide {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.side {
            KeySide::None => write!(f, "{}", self.letter),
            KeySide::Left => write!(f, "{}-", self.letter),
            KeySide::Right => write!(f, "-{}", self.letter),
        }
    }
}

impl LetterWithSide {
    /// Parses `<letter>`, `<letter>-` or `-<letter>`.
    pub fn parse(s: &str) -> Option<Self> {
        if s.len() > 8 {
            // Apparently too long.
            return None;
        }

        if s == "--" {
            return None;
        }

        let cs = s.chars().collect::<Vec<_>>();
        match cs.len() {
            0 => None,
            1 if s == "-" => None,
            1 => Some(Self {
                letter: cs[0],
                side: KeySide::None,
            }),
            2 if s == "--" => None,
            2 if s.ends_with('-') => Some(Self {
                letter: cs[0],
                side: KeySide::Left,
            }),
            2 if s.starts_with('-') => Some(Self {
                letter: cs[1],
                side: KeySide::Right,
            }),
            _ => None,
        }
    }
}

/// Bitmask of characters pressed on a steno keyboard (up to 63 keys).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Stroke {
    /// Bitmask
    pub mask: usize,
}

impl Stroke {
    pub fn new(mask: usize) -> Self {
        Self { mask }
    }

    pub fn is_empty(&self) -> bool {
        self.mask == 0
    }
}

/// A sequence of stenographic keyboard keys.
///
/// Different from the original `stroke`, our `StenoSystem` does not have number key
/// layout information, because they should be implemented as dictionary data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StenoSystem {
    /// 63 (constant), because we want to use bit flags of length `64`.
    n_max_keys: usize,
    /// n_max_keys + 1 (+1 for hyphen).
    n_max_steno: usize,
    /// A bitmask that filters a contiguous block of keys whose letters are unique and the side can
    /// be known without explicit specification by user.
    implicit_hyphen_mask: usize,
    /// Index of the first right side key.
    right_keys_index: usize,
    /// Array of length `n_keys` that maps key indices to letters.
    pub keys: Box<[LetterWithSide]>,
}

/// Error found in `StenoSystem::new`.
#[derive(Debug, Error)]
pub enum StenoSystemError {
    #[error("invalid implicit hyphen keys")]
    InvalidImplicitHyphenKeys,
    #[error("given out of range implicit hyphen key range")]
    OutOfRangeImplicitHyphenKeyRange,
    #[error("not all implicit hyphen keys are unique")]
    NotAllImplicitHyphenKeysUnique,
    #[error("found left side key in right side: {0}")]
    FoundLeftSideKeyInRightSide(char),
}

impl StenoSystem {
    /// _O(nk)_ Creates a stey system from user-friendly input.
    ///
    /// (_n_: number of keys, _k_: length of the `numbers` map)
    pub fn new(
        letters: &[LetterWithSide],
        implicit_hyphen_key_range: Option<ops::Range<usize>>,
    ) -> Result<Self, StenoSystemError> {
        let implicit_hyphen_key_range = implicit_hyphen_key_range.unwrap_or(0..0);

        // Now, for simplicity, I'll iterate through the `keys_input` multple times.
        let n_keys = letters.len();
        let keys = Vec::<LetterWithSide>::from(letters);

        // Find the first right-side key index:
        let right_keys_index = letters
            .iter()
            .position(|LetterWithSide { side, .. }| *side == KeySide::Right)
            .unwrap_or(n_keys);

        // Left hand side keys must not appear after the first right side key:
        if let Some(k) = letters
            .iter()
            .skip(right_keys_index + 1)
            .cloned()
            .find(|k| k.side == KeySide::Left)
        {
            return Err(StenoSystemError::FoundLeftSideKeyInRightSide(k.letter));
        }

        // TODO: Use it for validation
        let unique_key_mask = crate::util::retain_unique_indices(
            letters.iter().map(|k| k.letter).collect::<Vec<_>>(),
        )
        .iter()
        .fold(0usize, |mask, i| mask | (1 << i));

        let implicit_hyphen_mask = {
            if !crate::util::contains_range(0..keys.len(), implicit_hyphen_key_range.clone()) {
                return Err(StenoSystemError::OutOfRangeImplicitHyphenKeyRange);
            }

            // Create implicit hyphen key mask with an assumption that all of the keys are unique
            // (ensured later)
            let implicit_hyphen_mask = implicit_hyphen_key_range
                .clone()
                .into_iter()
                .fold(0usize, |mask, i| mask | (1 << i));

            // All of the implicit hyphen keys must be unique so that we can auto insert hyphens
            if (implicit_hyphen_mask & unique_key_mask) != implicit_hyphen_mask {
                return Err(StenoSystemError::NotAllImplicitHyphenKeysUnique);
            }

            implicit_hyphen_mask
        };

        Ok(Self {
            n_max_keys: 63,
            n_max_steno: 64,
            implicit_hyphen_mask,
            right_keys_index,
            keys: keys.into_boxed_slice(),
        })
    }

    /// _O(min(64, n))_ Creates a `Stroke` from a `String` notation of it. This function is strict
    /// about steno order, meaning that the stroke notation is basically a subsequence of
    /// `StenoSystem` keys.
    ///
    /// Say `ABCDE` is our `StenoSystem` keys and `AE` is the input stroke notation. `AE` is clearly
    /// a subsequence:
    ///
    /// ```txt
    /// A B C D E
    /// A       E
    /// ```
    ///
    /// ```
    /// use qlover::model::stroke::{LetterWithSide, StenoSystem, Stroke};
    /// let keys = "A- B- C -D -E"
    ///     .split_whitespace()
    ///     .map(LetterWithSide::parse)
    ///     .collect::<Option<Vec<_>>>()
    ///     .unwrap();
    /// let system = StenoSystem::new(&keys, Some(2..3)).unwrap();
    /// assert_eq!(system.parse_stroke("AE"), Some(Stroke { mask: 17 }));
    /// ```
    pub fn parse_stroke(self, stroke_notation: &str) -> Option<Stroke> {
        // Bitmask of keys  the `stroke` presses.
        let mut mask = 0;

        let mut current_key_index = 0;

        for c in stroke_notation.chars() {
            // Meta key handling
            match c {
                '-' => match current_key_index.cmp(&self.right_keys_index) {
                    // unreachable:
                    Ordering::Greater => return None,
                    // Equal: consecutive heyphens are allowed, right?
                    Ordering::Equal | Ordering::Less => {
                        current_key_index = self.right_keys_index;
                        continue;
                    }
                },
                _ => {}
            }

            // Find next character that matches to the input, in steno order.
            // while let Some(key) = self.keys.get(current_key_index)
            while matches!(
                self.keys.get(current_key_index),
                Some(key) if key.letter != c,
            ) {
                current_key_index += 1;
            }

            if current_key_index == self.keys.len() {
                // Failed to find the next key.
                return None;
            }

            // This key is pressed.
            mask |= 1 << current_key_index
        }

        // We could remove this check, but let's prefer safety:
        Some(
            self.stroke_from_bitmask(mask)
                .unwrap_or_else(|| unreachable!()),
        )
    }

    /// _O(1)_ Creates a [`Stroke`] from a bitmask, performing boundary check.
    pub fn stroke_from_bitmask(&self, mask: usize) -> Option<Stroke> {
        if mask >> self.keys.len() == 0 {
            Some(Stroke { mask })
        } else {
            None
        }
    }

    /// _O(kn)_ Creates [`Stroke`] from a slice of [`LetterWithSide`]. The function is not strict
    /// about steno order and just sums up the given keys to the resulting [`Stroke`].
    pub fn stroke_from_keys(&self, keys: &[LetterWithSide]) -> Option<Stroke> {
        let mut mask = 0;

        // TODO: rev?
        for LetterWithSide { letter, side } in keys.iter().rev().cloned() {
            let (start, end) = match side {
                KeySide::None => (0, self.keys.len()),
                KeySide::Left => (0, self.right_keys_index),
                KeySide::Right => (self.right_keys_index, self.keys.len()),
            };

            // TODO: It should be more strict about steno order, e.g., it accepts `A- -A
            // A-`? It is the design of original `stroke` though.
            if let Some(k) = self.keys[start..end]
                .iter()
                .position(|steno_key| steno_key.letter == letter && steno_key.side == side)
                .map(|i| i + start)
            {
                mask |= 1 << k;
            } else {
                return None;
            }
        }

        Some(Stroke { mask })
    }
}

/// Parses steno keys from a string notation.
///
/// # Example
///
/// English steno key can be parsed as follows:
///
/// ```
/// let s = r##"
///     #
///     S- T- K- P- W- H- R-
///     A- O-
///     *
///     -E -U
///     -F -R -P -B -L -G -T -S -D -Z
/// "##;
/// let _ = qlover::model::stroke::parse_keys(s);
/// ```
pub fn parse_keys(s: &str) -> Option<Vec<LetterWithSide>> {
    s.split_whitespace()
        .map(LetterWithSide::parse)
        .collect::<Option<Vec<_>>>()
}

/// Parses a steno system from a string notation.
///
/// # Example
///
/// English steno key can be parsed as follows:
///
/// ```
/// let s = r##"
///     #
///     S- T- K- P- W- H- R-
///     A- O-
///     *
///     -E -U
///     -F -R -P -B -L -G -T -S -D -Z
/// "##;
///
/// // A-, O-, *, -E, -U are the implicit hyphen keys (unique keys where the side is deterministic):
/// let r = 8..13;
///
/// let _ = qlover::model::stroke::parse_system(s, Some(8..13));
/// ```
pub fn parse_system(
    s: &str,
    rng: Option<ops::Range<usize>>,
) -> Option<Result<StenoSystem, StenoSystemError>> {
    let keys = self::parse_keys(s)?;
    Some(StenoSystem::new(&keys, rng))
}
