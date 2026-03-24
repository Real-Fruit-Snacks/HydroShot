use hydroshot::config::Config;
use hydroshot::geometry::Color;

#[test]
fn default_config_has_correct_values() {
    let cfg = Config::default();
    assert_eq!(cfg.general.default_color, "red");
    assert_eq!(cfg.general.default_thickness, 3.0);
    assert_eq!(cfg.general.save_directory, "");
    assert_eq!(cfg.hotkey.capture, "Ctrl+Shift+S");
}

#[test]
fn parse_color_valid() {
    let mut cfg = Config::default();
    cfg.general.default_color = "blue".to_string();
    assert_eq!(cfg.default_color(), Color::blue());

    cfg.general.default_color = "green".to_string();
    assert_eq!(cfg.default_color(), Color::green());

    cfg.general.default_color = "yellow".to_string();
    assert_eq!(cfg.default_color(), Color::yellow());

    cfg.general.default_color = "white".to_string();
    assert_eq!(cfg.default_color(), Color::white());

    cfg.general.default_color = "red".to_string();
    assert_eq!(cfg.default_color(), Color::red());
}

#[test]
fn parse_color_invalid_falls_back_to_red() {
    let mut cfg = Config::default();
    cfg.general.default_color = "magenta".to_string();
    assert_eq!(cfg.default_color(), Color::red());

    cfg.general.default_color = "".to_string();
    assert_eq!(cfg.default_color(), Color::red());
}

#[test]
fn serialize_roundtrip_via_toml() {
    let cfg = Config::default();
    let serialized = toml::to_string(&cfg).expect("serialize");
    let deserialized: Config = toml::from_str(&serialized).expect("deserialize");
    assert_eq!(deserialized.general.default_color, cfg.general.default_color);
    assert_eq!(deserialized.general.default_thickness, cfg.general.default_thickness);
    assert_eq!(deserialized.general.save_directory, cfg.general.save_directory);
    assert_eq!(deserialized.hotkey.capture, cfg.hotkey.capture);
}

#[test]
fn thickness_clamping() {
    let mut cfg = Config::default();

    cfg.general.default_thickness = 100.0;
    assert_eq!(cfg.clamped_thickness(), 20.0);

    cfg.general.default_thickness = -5.0;
    assert_eq!(cfg.clamped_thickness(), 1.0);

    cfg.general.default_thickness = 10.0;
    assert_eq!(cfg.clamped_thickness(), 10.0);
}
