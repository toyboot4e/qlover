use proptest::prelude::*;
use rustc_hash::FxHashMap;
use test_strategy::proptest;

use crate::model::dictionary::{Outline, Output, StenoDictionary};

fn stroke_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[A-Z]+").unwrap()
}

impl Arbitrary for Outline {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        prop::collection::vec(stroke_strategy(), 1..=4)
            .prop_map(Outline)
            .boxed()
    }
}

impl Arbitrary for Output {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        ".+".prop_map(Output::String).boxed()
    }
}

#[proptest]
fn outline_serde_roundtrip(outline: Outline) {
    let json = serde_json::to_string(&outline).unwrap();
    let roundtripped: Outline = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped, outline);
}

#[proptest]
fn outline_display_joins_with_slash(outline: Outline) {
    let expected = outline.0.join("/");
    assert_eq!(format!("{}", outline), expected);
}

#[proptest]
fn dictionary_get_returns_inserted(
    #[strategy(prop::collection::vec(any::<(Outline, Output)>(), 1..=20))] entries: Vec<(
        Outline,
        Output,
    )>,
) {
    let map: FxHashMap<Outline, Output> = entries.iter().cloned().collect();
    let dict = StenoDictionary::new(map.clone());
    for (outline, output) in &map {
        assert_eq!(dict.get(outline), Some(output));
    }
}

#[proptest]
fn dictionary_serde_roundtrip(
    #[strategy(prop::collection::vec(any::<(Outline, Output)>(), 1..=20))] entries: Vec<(
        Outline,
        Output,
    )>,
) {
    let map: FxHashMap<Outline, Output> = entries.into_iter().collect();
    let dict = StenoDictionary::new(map);
    let json = serde_json::to_string(&dict).unwrap();
    let roundtripped: StenoDictionary = serde_json::from_str(&json).unwrap();
    let json2 = serde_json::to_string(&roundtripped).unwrap();
    let v1: serde_json::Value = serde_json::from_str(&json).unwrap();
    let v2: serde_json::Value = serde_json::from_str(&json2).unwrap();
    assert_eq!(v1, v2);
}
