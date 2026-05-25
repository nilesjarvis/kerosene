use super::{
    CustomFontConfig, DisplayFontConfig, KeroseneConfig, default_config_value, json_string,
    object_mut, value_from_json, value_from_str,
};

#[test]
fn display_and_monospace_fonts_round_trip_and_legacy_default_system() {
    let config = KeroseneConfig {
        display_font: DisplayFontConfig::Custom {
            family: "Inter".to_string(),
        },
        monospace_font: DisplayFontConfig::Custom {
            family: "Roboto Mono".to_string(),
        },
        custom_fonts: vec![CustomFontConfig {
            family: "Inter".to_string(),
            file_name: "inter.ttf".to_string(),
        }],
        ..KeroseneConfig::default()
    };

    let json = json_string(&config, "config should serialize");
    let decoded: KeroseneConfig = value_from_str(&json, "config should deserialize");
    assert_eq!(decoded.display_font, config.display_font);
    assert_eq!(decoded.monospace_font, config.monospace_font);
    assert_eq!(decoded.custom_fonts, config.custom_fonts);

    let mut legacy = default_config_value();
    let object = object_mut(&mut legacy, "config should serialize to object");
    object.remove("display_font");
    object.remove("monospace_font");
    object.remove("custom_fonts");

    let decoded_legacy: KeroseneConfig =
        value_from_json(legacy, "legacy config should deserialize");
    assert_eq!(decoded_legacy.display_font, DisplayFontConfig::System);
    assert_eq!(decoded_legacy.monospace_font, DisplayFontConfig::System);
    assert!(decoded_legacy.custom_fonts.is_empty());
}

#[test]
fn bundled_display_and_monospace_fonts_do_not_require_custom_font_entries() {
    for family in crate::config::BUNDLED_DISPLAY_FONT_FAMILIES {
        let config = KeroseneConfig {
            display_font: DisplayFontConfig::Custom {
                family: family.to_ascii_lowercase(),
            },
            monospace_font: DisplayFontConfig::Custom {
                family: family.to_ascii_lowercase(),
            },
            custom_fonts: Vec::new(),
            ..KeroseneConfig::default()
        };
        let custom_fonts = crate::config::normalize_custom_fonts(config.custom_fonts);
        let display_font =
            crate::config::normalize_display_font(config.display_font, &custom_fonts);
        let monospace_font =
            crate::config::normalize_display_font(config.monospace_font, &custom_fonts);

        assert_eq!(
            display_font,
            DisplayFontConfig::Custom {
                family: (*family).to_string()
            }
        );
        assert_eq!(
            monospace_font,
            DisplayFontConfig::Custom {
                family: (*family).to_string()
            }
        );
    }
}
