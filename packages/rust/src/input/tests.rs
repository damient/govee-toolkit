use super::*;
use crate::apply::Verb;
use crate::codec::coerce::Supplied;
use crate::stream::{Rate, Resolution};

fn ints(values: &[i64]) -> Value {
    Value::List(values.iter().copied().map(Value::Int).collect())
}

fn map(entries: Vec<(&str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    )
}

#[test]
fn a_color_is_three_channels_or_three_bytes() {
    assert_eq!(rgb(&ints(&[255, 0, 0]), Style::Python), Ok([255, 0, 0]));
    assert_eq!(
        rgb(&Value::Bytes(vec![1, 2, 3]), Style::Javascript),
        Ok([1, 2, 3])
    );
    assert!(rgb(&ints(&[255, 0]), Style::Python).is_err());
    assert!(rgb(&ints(&[256, 0, 0]), Style::Python).is_err());
}

#[test]
fn a_paint_reads_one_color_a_list_or_packed_bytes() {
    assert_eq!(
        colors(&ints(&[1, 2, 3]), Style::Python),
        Ok(vec![[1, 2, 3]])
    );
    let list = Value::List(vec![ints(&[1, 2, 3]), ints(&[4, 5, 6])]);
    assert_eq!(colors(&list, Style::Python), Ok(vec![[1, 2, 3], [4, 5, 6]]));
    let packed = Value::Bytes(vec![1, 2, 3, 4, 5, 6]);
    assert_eq!(
        colors(&packed, Style::Python),
        Ok(vec![[1, 2, 3], [4, 5, 6]])
    );
    assert!(colors(&Value::Bytes(vec![1, 2, 3, 4]), Style::Python).is_err());
}

#[test]
fn a_boolean_is_no_rate_and_no_resolution() {
    assert!(rate(&Value::Bool(true)).is_err());
    assert!(resolution(&Value::Bool(true)).is_err());
    assert_eq!(rate(&Value::Null), Ok(Rate::default()));
    assert_eq!(rate(&Value::Int(30)), Ok(Rate::Fixed(30.0)));
    assert_eq!(resolution(&Value::Int(12)), Ok(Resolution::Exact(12)));
    assert_eq!(
        resolution(&Value::Text("groups".into())),
        Ok(Resolution::Groups)
    );
}

#[test]
fn an_argument_reads_its_shape() {
    assert_eq!(
        supplied(&Value::Bool(true), Style::Python),
        Ok(Supplied::Int(1))
    );
    assert!(supplied(&Value::Float(1.5), Style::Javascript).is_err());
    assert!(supplied(&Value::Null, Style::Python).is_err());
    let rows = Value::List(vec![ints(&[255, 0, 0])]);
    assert_eq!(
        supplied(&rows, Style::Python),
        Ok(Supplied::Colors(vec![[255, 0, 0]]))
    );
    assert_eq!(
        supplied(&ints(&[0, 1]), Style::Python),
        Ok(Supplied::Ints(vec![0, 1]))
    );
}

#[test]
fn the_verbs_take_the_host_spelling_and_refuse_a_misspelled_key() {
    let js = map(vec![
        ("colorTemp", Value::Int(2700)),
        ("power", Value::Bool(true)),
    ]);
    assert_eq!(
        verbs(&js, Style::Javascript),
        Ok(vec![Verb::Power(true), Verb::ColorTemp(2700)])
    );
    let py = map(vec![("color_temp", Value::Int(2700))]);
    assert_eq!(verbs(&py, Style::Python), Ok(vec![Verb::ColorTemp(2700)]));
    assert!(verbs(&map(vec![("colour", Value::Null)]), Style::Python).is_err());
    let segment = map(vec![("segment", map(vec![("zones", ints(&[0]))]))]);
    assert!(verbs(&segment, Style::Python).is_err());
}

#[test]
fn a_walk_keeps_its_defaults_and_refuses_a_negative_wait() {
    assert!(walk(&Value::Null, Style::Python).is_ok());
    assert!(walk(&map(vec![("wait", Value::Float(-1.0))]), Style::Python).is_err());
    assert!(walk(&map(vec![("speed", Value::Int(1))]), Style::Python).is_err());
}
