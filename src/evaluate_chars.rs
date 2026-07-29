use aviutl2_text_parser::Element;

#[derive(Debug, Clone, PartialEq)]
pub enum TextUnit {
    Char(char),
    Emoji {
        name: String,
    },
    Ruby {
        base: Vec<Element>,
        ruby: Vec<Element>,
        scale: Option<f64>,
        expand_line_height: bool,
    },
}

impl TextUnit {
    pub fn as_char(&self) -> Option<char> {
        match self {
            Self::Char(value) => Some(*value),
            Self::Emoji { .. } | Self::Ruby { .. } => None,
        }
    }

    pub fn is_whitespace(&self) -> bool {
        self.as_char().is_some_and(char::is_whitespace)
    }

    pub fn segmentation_text(&self) -> String {
        match self {
            Self::Char(value) => value.to_string(),
            Self::Emoji { .. } => "\u{fffc}".to_string(),
            Self::Ruby { base, .. } => {
                let text = elements_to_segmentation_text(base);
                if text.is_empty() {
                    "\u{fffc}".to_string()
                } else {
                    text
                }
            }
        }
    }
}

impl std::fmt::Display for TextUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Char(value) => write!(f, "{value}"),
            Self::Emoji { name } => write!(f, "{}", Element::Emoji { name: name.clone() }),
            Self::Ruby {
                base,
                ruby,
                scale,
                expand_line_height,
            } => write!(
                f,
                "{}",
                Element::Ruby {
                    base: base.clone(),
                    ruby: ruby.clone(),
                    scale: *scale,
                    expand_line_height: *expand_line_height,
                }
            ),
        }
    }
}

fn elements_to_segmentation_text(elements: &[Element]) -> String {
    elements
        .iter()
        .map(|element| match element {
            Element::Text(text) => text.clone(),
            Element::Emoji { .. } => "\u{fffc}".to_string(),
            Element::Ruby { base, .. } => elements_to_segmentation_text(base),
            _ => String::new(),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharState {
    pub unit: TextUnit,
    pub control_index: usize,
    pub start_time: f64,
}

#[derive(Debug)]
pub struct EvaluatedChars {
    pub chars: Vec<CharState>,
    pub controls: Vec<Element>,
}

fn is_timing_control(control: &Element) -> bool {
    matches!(control, Element::Speed { .. } | Element::Wait { .. })
}

pub fn controls_to_text(controls: &[Element], end: usize) -> String {
    controls_between_to_text(controls, 0, end)
}

pub fn controls_between_to_text(controls: &[Element], start: usize, end: usize) -> String {
    controls[start..end]
        .iter()
        .filter(|control| !is_timing_control(control))
        .map(ToString::to_string)
        .collect()
}

pub fn char_states_to_text(char_states: &[CharState], controls: &[Element], time: f64) -> String {
    let Some(first) = char_states.first() else {
        return String::new();
    };

    let mut result = controls_to_text(controls, first.control_index);
    let mut emitted_controls = first.control_index;
    for char_state in char_states {
        if char_state.start_time > time {
            break;
        }
        result.extend(
            controls[emitted_controls..char_state.control_index]
                .iter()
                .filter(|control| !is_timing_control(control))
                .map(ToString::to_string),
        );
        result.push_str(&char_state.unit.to_string());
        emitted_controls = char_state.control_index;
    }
    result
}

pub fn evaluate_chars(
    text: &str,
    initial_controls: Vec<Element>,
    base_speed: f64,
) -> EvaluatedChars {
    let parsed = aviutl2_text_parser::parse_control(text);
    let mut chars = Vec::new();
    let mut controls = initial_controls;
    let mut current_speed = base_speed;
    let mut start_time = 0.0;
    let mut num_chars = 0;

    for item in parsed {
        let inv_speed = if current_speed == 0.0 {
            0.0
        } else {
            1.0 / current_speed
        };
        match item {
            Element::Text(text) => {
                for value in text.chars() {
                    chars.push(CharState {
                        unit: TextUnit::Char(value),
                        control_index: controls.len(),
                        start_time,
                    });
                    if value != '\t' && value != '\n' {
                        start_time += inv_speed;
                        num_chars += 1;
                    }
                }
            }
            Element::Emoji { name } => {
                chars.push(CharState {
                    unit: TextUnit::Emoji { name },
                    control_index: controls.len(),
                    start_time,
                });
                start_time += inv_speed;
                num_chars += 1;
            }
            Element::Ruby {
                base,
                ruby,
                scale,
                expand_line_height,
            } => {
                chars.push(CharState {
                    unit: TextUnit::Ruby {
                        base,
                        ruby,
                        scale,
                        expand_line_height,
                    },
                    control_index: controls.len(),
                    start_time,
                });
                start_time += inv_speed;
                num_chars += 1;
            }
            Element::Speed { speed } => {
                current_speed = speed.unwrap_or(base_speed);
                controls.push(Element::Speed { speed });
            }
            Element::Wait { time } => {
                match time {
                    aviutl2_text_parser::TimeValue::Absolute(value) => {
                        start_time += value + inv_speed;
                    }
                    aviutl2_text_parser::TimeValue::PerChar(value) => {
                        start_time += value * num_chars as f64 + inv_speed;
                    }
                }
                controls.push(Element::Wait { time });
            }
            control => controls.push(control),
        }
    }

    EvaluatedChars { chars, controls }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_controls_and_restores_ranges() {
        let evaluated = evaluate_chars("A<#ff0000>B<p+10>C", vec![], 0.0);
        assert_eq!(evaluated.chars.len(), 3);
        assert_eq!(
            char_states_to_text(&evaluated.chars[1..], &evaluated.controls, f64::INFINITY),
            "<#ff0000>B<p+10>C"
        );
    }

    #[test]
    fn treats_emoji_and_ruby_as_single_units() {
        let evaluated = evaluate_chars("</>制御文字<!0.4+>せいぎょもじ</><&いいね>B", vec![], 1.0);
        assert_eq!(evaluated.chars.len(), 3);
        assert!(matches!(evaluated.chars[0].unit, TextUnit::Ruby { .. }));
        assert!(matches!(evaluated.chars[1].unit, TextUnit::Emoji { .. }));
        assert_eq!(evaluated.chars[2].start_time, 2.0);
        assert_eq!(evaluated.chars[0].unit.segmentation_text(), "制御文字");
    }

    #[test]
    fn timing_controls_are_not_rendered() {
        let evaluated = evaluate_chars("A<r2>B<w1>C", vec![], 1.0);
        assert_eq!(evaluated.chars[1].start_time, 1.0);
        assert_eq!(evaluated.chars[2].start_time, 3.0);
        assert_eq!(
            char_states_to_text(&evaluated.chars, &evaluated.controls, f64::INFINITY),
            "ABC"
        );
    }

    #[test]
    fn tab_and_newline_do_not_advance_time() {
        let evaluated = evaluate_chars("Line1\nLine2\tTabbed", vec![], 1.0);
        assert_eq!(evaluated.chars.len(), 18);
        assert_eq!(evaluated.chars[6].start_time, 5.0);
        assert_eq!(evaluated.chars[12].start_time, 10.0);
    }
}
