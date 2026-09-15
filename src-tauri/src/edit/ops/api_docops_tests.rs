use super::*;

#[test]
fn set_trim_replaces_trim() {
    let mut doc = empty();
    apply(
        &mut doc,
        EditOp::SetTrim {
            in_ms: 200,
            out_ms: 8000,
        },
    );
    assert_eq!(
        doc.trim,
        Trim {
            in_ms: 200,
            out_ms: 8000
        }
    );
}

#[test]
fn set_aspect_replaces_aspect() {
    use crate::export::types::Aspect;
    let mut doc = empty();
    assert_eq!(doc.aspect, Aspect::Source);
    apply(
        &mut doc,
        EditOp::SetAspect {
            aspect: Aspect::Vertical9x16,
        },
    );
    assert_eq!(doc.aspect, Aspect::Vertical9x16);
}
