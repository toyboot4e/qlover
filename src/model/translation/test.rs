use crate::model::{
    stroke::Stroke,
    translation::{Translation, Translator},
};

#[test]
fn test_simple() {
    let mut tr = Translator::new();
    let t = tr.translate(&[Stroke::new(0)]);
    assert_eq!(
        t,
        Translation {
            translated: "dummy".to_string()
        }
    );
}
