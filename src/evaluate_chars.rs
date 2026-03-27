#[derive(Debug, Clone)]
pub struct CharState {
    char: char,
    bold: bool,
    italic: bool,
    strikethrough: bool,
    size: f64,
    color: String,
    font: String,
    start_time: f64,
    end_time: Option<f64>,
}

pub fn evaluate_chars(
    text: &str,
    base_state: &CharState,
    base_speed: f64,
) -> anyhow::Result<Vec<CharState>> {
    let parsed = aviutl2_text_parser::parse_control(text);
    let mut chars = Vec::new();
    let mut current_state = base_state.clone();
    let mut current_speed = base_speed;
    let mut num_chars = 0;
    for item in parsed {
    let inv_speed = if current_speed == 0.0 {
        0.0
    } else {
        1.0 / current_speed
    };
        match item {
            aviutl2_text_parser::Element::Text(text) => {
                for c in text.chars() {
                    chars.push(CharState {
                        char: c,
                        ..current_state.clone()
                    });
                    current_state.start_time += inv_speed;
                    num_chars += 1;
                }
            }
            aviutl2_text_parser::Element::Color { code } => {
                current_state.color = match code {
                    aviutl2_text_parser::ColorType::Default => base_state.color.clone(),
                    aviutl2_text_parser::ColorType::Single(color_value) => color_value.to_string(),
                    aviutl2_text_parser::ColorType::Pair(color_value, _) => color_value.to_string(),
                };
            }
            aviutl2_text_parser::Element::Size {
                size,
                font,
                decoration,
                outline_size,
            } => {
                current_state.size = match size {
                    aviutl2_text_parser::ScalarValue::Default => current_state.size,
                    aviutl2_text_parser::ScalarValue::Absolute(value) => value,
                    aviutl2_text_parser::ScalarValue::RelativeAdd(value) => {
                        current_state.size + value
                    }
                    aviutl2_text_parser::ScalarValue::RelativeMul(value) => {
                        current_state.size * value
                    }
                };
                if let Some(font) = font {
                    current_state.font = font;
                }
                if let Some(decoration) = decoration {
                    current_state.bold = decoration.bold;
                    current_state.italic = decoration.italic;
                    current_state.strikethrough = decoration.strikethrough;
                }
            }
            aviutl2_text_parser::Element::Font { name } => current_state.font = name,
            aviutl2_text_parser::Element::Speed { speed } => match speed {
                Some(speed) => current_speed = speed,
                None => current_speed = base_speed,
            },
            aviutl2_text_parser::Element::Wait { time } => match time {
                aviutl2_text_parser::TimeValue::Absolute(v) => {
                    current_state.start_time += v + inv_speed
                }
                aviutl2_text_parser::TimeValue::PerChar(v) => {
                    current_state.start_time += v * num_chars as f64 + inv_speed
                }
            },
            aviutl2_text_parser::Element::Clear { time } => {
                let mut clear_at = match time {
                    aviutl2_text_parser::TimeValue::Absolute(v) => {
                        current_state.start_time + v + inv_speed
                    }
                    aviutl2_text_parser::TimeValue::PerChar(v) => {
                        current_state.start_time + v * num_chars as f64 + inv_speed
                    }
                };

                for char_state in chars.iter_mut().rev() {
                    if char_state.end_time.is_none() {
                        char_state.end_time = Some(clear_at);
                    } else {
                        break;
                    }
                }
            }
            aviutl2_text_parser::Element::Position { x, y, z } => {
                anyhow::bail!("Position control is not supported");
            }
            aviutl2_text_parser::Element::Script { code } => {
                anyhow::bail!("Script control is not supported");
            }
        }
    }

    Ok(chars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_chars() {
        let base_state = CharState {
            char: ' ',
            bold: false,
            italic: false,
            strikethrough: false,
            size: 12.0,
            color: "white".to_string(),
            font: "Arial".to_string(),
            start_time: 0.0,
            end_time: None,
        };
        let text = "Hello<#red>W<r2.0>orld<s1>!";
        let chars = evaluate_chars(text, &base_state, 0.0).unwrap();
        assert_eq!(chars.len(), 11);
        assert_eq!(chars[0].char, 'H');
        assert_eq!(chars[5].char, 'W');
        assert_eq!(chars[5].color, "red");
        assert_eq!(chars[5].start_time, 0.0);
        assert_eq!(chars[7].char, 'r');
        assert_eq!(chars[7].start_time, 0.5);
        assert_eq!(chars[10].char, '!');
    }
}
