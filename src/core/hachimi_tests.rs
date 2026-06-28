#[cfg(test)]
mod tests {
    use super::super::core::hachimi::{caption_config_fields, default_config_field_order, CaptionConfig, Config};

    #[test]
    fn default_config_field_order_contains_caption_fields() {
        let order = default_config_field_order();
        let caption_fields = caption_config_fields();

        for field in caption_fields {
            assert!(order.contains(&field), "caption field '{}' is missing from canonical order", field);
        }
    }

    #[test]
    fn caption_config_fields_match_struct_fields() {
        let fields = caption_config_fields();
        let expected = vec![
            "caption_enable",
            "caption_show_log_enable",
            "caption_format_log_enable",
            "caption_fallback_enable",
            "caption_lines_char_count",
            "caption_font_size",
            "caption_color",
            "caption_outline_size",
            "caption_outline_color",
            "caption_bg_alpha",
            "caption_pos_x",
            "caption_pos_y",
        ];

        assert_eq!(fields, expected, "caption config fields changed unexpectedly");
    }

    #[test]
    fn config_default_is_serializable() {
        let _ = serde_json::to_value(&Config::default()).expect("default Config must serialize");
    }

    #[test]
    fn caption_config_default_is_serializable() {
        let _ = serde_json::to_value(&CaptionConfig::default()).expect("default CaptionConfig must serialize");
    }
}