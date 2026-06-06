use crate::model::stroke::{self, KeySide, LetterWithSide, StenoSystem};

fn english_system_keys() -> Vec<LetterWithSide> {
    let s = r##"
        #
        S- T- K- P- W- H- R-
        A- O-
        *
        -E -U
        -F -R -P -B -L -G -T -S -D -Z
"##;

    stroke::parse_keys(s).unwrap()
}

fn english_system() -> StenoSystem {
    StenoSystem::new(&english_system_keys(), Some(8..13)).unwrap()
}

#[test]
fn test_minimal() {
    let system = english_system();

    assert_eq!(
        system.right_keys_index,
        english_system_keys()
            .iter()
            .position(|l| *l
                == LetterWithSide {
                    letter: 'E',
                    side: KeySide::Right
                })
            .unwrap(),
    );

    assert_eq!(
        system.implicit_hyphen_mask,
        (8..13).into_iter().fold(0usize, |acc, i| acc | (1 << i))
    );
}
