use super::*;

fn parse(s: &str) -> TargetId {
    s.parse::<TargetId>().unwrap()
}

#[test]
fn displays_round_trip_through_the_display_n_form() {
    assert_eq!(parse("display:0"), TargetId::Display(0));
    assert_eq!(parse("display:2"), TargetId::Display(2));
    assert_eq!(TargetId::Display(0).to_string(), "display:0");
    assert_eq!(TargetId::Display(2).to_string(), "display:2");
    assert_eq!(TargetId::Display(2).as_arg().as_deref(), Some("display:2"));
}

#[test]
fn windows_round_trip_through_the_lowercase_hex_hwnd_form() {
    assert_eq!(parse("window:0x1"), TargetId::Window(1));
    assert_eq!(parse("window:0x1a2b3c"), TargetId::Window(0x1a_2b_3c));
    assert_eq!(TargetId::Window(0x1a_2b_3c).to_string(), "window:0x1a2b3c");
    assert_eq!(TargetId::Window(1).as_arg().as_deref(), Some("window:0x1"));
}

#[test]
fn the_primary_display_is_none_as_an_argument_and_primary_as_a_string() {
    assert_eq!(TargetId::from_arg(None), TargetId::Primary);
    assert_eq!(TargetId::Primary.as_arg(), None);
    assert_eq!(TargetId::Primary.to_string(), "primary");
    assert_eq!(parse("primary"), TargetId::Primary);
}

#[test]
fn anything_unparseable_falls_back_to_primary_the_way_every_parse_site_does() {
    for s in [
        "",
        "display:",
        "display:x",
        "window:0x",
        "window:0xzz",
        "window:1",
        "nonsense",
    ] {
        assert_eq!(parse(s), TargetId::Primary, "{s}");
    }
    assert_eq!(TargetId::from_arg(Some("display:x")), TargetId::Primary);
}

#[test]
fn every_arg_form_survives_a_trip_out_and_back() {
    for id in [
        TargetId::Primary,
        TargetId::Display(0),
        TargetId::Display(7),
        TargetId::Window(0xfeed),
    ] {
        assert_eq!(TargetId::from_arg(id.as_arg().as_deref()), id);
    }
}
