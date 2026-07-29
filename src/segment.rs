use crate::evaluate_chars::CharState;

#[derive(Debug, Clone, PartialEq)]
pub enum WrappedBy {
    Whitespace(Vec<CharState>),
    Budoux,
    Manual,
    Overflow,
    None,
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub chars: Vec<CharState>,
    pub wrapped_by: WrappedBy,
}

pub fn segment_manually(char_states: &[CharState]) -> Vec<Segment> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < char_states.len() {
        if char_states[i].unit.as_char() == Some('\\')
            && i + 1 < char_states.len()
            && char_states[i + 1].unit.as_char() == Some('b')
        {
            result.push(Segment {
                chars: char_states[start..i].to_vec(),
                wrapped_by: if start == 0 {
                    WrappedBy::None
                } else {
                    WrappedBy::Manual
                },
            });
            start = i + 2;
            i += 2;
        } else {
            i += 1;
        }
    }
    result.push(Segment {
        chars: char_states[start..].to_vec(),
        wrapped_by: if start == 0 {
            WrappedBy::None
        } else {
            WrappedBy::Manual
        },
    });
    result
}

pub fn segment_with_budoux(char_states: &[CharState]) -> Vec<Segment> {
    let mut owners = Vec::new();
    let text: String = char_states
        .iter()
        .enumerate()
        .map(|(index, state)| {
            let text = state.unit.segmentation_text();
            owners.extend(std::iter::repeat_n(index, text.chars().count()));
            text
        })
        .collect();
    let text_segments = crate::budoux::segment(&text);
    let mut result = Vec::new();
    let mut char_index = 0;
    let mut unit_index = 0;

    for text_segment in text_segments {
        char_index += text_segment.chars().count();
        if char_index < owners.len() && owners[char_index - 1] == owners[char_index] {
            continue;
        }
        let end_unit = if char_index == owners.len() {
            char_states.len()
        } else {
            owners[char_index]
        };
        result.push(Segment {
            chars: char_states[unit_index..end_unit].to_vec(),
            wrapped_by: if unit_index == 0 {
                WrappedBy::None
            } else {
                WrappedBy::Budoux
            },
        });
        unit_index = end_unit;
    }

    result
}

pub fn segment_with_whitespace(char_states: &[CharState]) -> Vec<Segment> {
    let mut result = Vec::new();
    let mut run_start: Option<usize> = None;
    let mut pending_whitespace: Vec<CharState> = Vec::new();

    for (i, char_state) in char_states.iter().enumerate() {
        if char_state.unit.is_whitespace() {
            if let Some(start) = run_start.take() {
                let wrapped_by = if pending_whitespace.is_empty() {
                    WrappedBy::None
                } else {
                    WrappedBy::Whitespace(std::mem::take(&mut pending_whitespace))
                };
                result.push(Segment {
                    chars: char_states[start..i].to_vec(),
                    wrapped_by,
                });
            }
            pending_whitespace.push(char_state.clone());
            continue;
        }

        if run_start.is_none() {
            run_start = Some(i);
        }
    }

    if let Some(start) = run_start {
        let wrapped_by = if pending_whitespace.is_empty() {
            WrappedBy::None
        } else {
            WrappedBy::Whitespace(std::mem::take(&mut pending_whitespace))
        };
        result.push(Segment {
            chars: char_states[start..].to_vec(),
            wrapped_by,
        });
    }

    if !pending_whitespace.is_empty() && !result.is_empty() {
        result.last_mut().expect("result is not empty").wrapped_by =
            WrappedBy::Whitespace(pending_whitespace);
    }

    result
}

pub fn segment(char_states: &[CharState]) -> Vec<Segment> {
    segment_with_whitespace(char_states)
        .into_iter()
        .flat_map(|segment| {
            let mut segments = segment_manually(&segment.chars);
            if let Some(first) = segments.first_mut() {
                first.wrapped_by = segment.wrapped_by;
            }
            segments
        })
        .flat_map(|segment| {
            let mut segments = segment_with_budoux(&segment.chars);
            if let Some(first) = segments.first_mut() {
                first.wrapped_by = segment.wrapped_by;
            }
            segments
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_char_state(c: char) -> CharState {
        CharState {
            unit: crate::evaluate_chars::TextUnit::Char(c),
            control_index: 0,
            start_time: 0.0,
        }
    }

    fn make_ruby_state(base: &str, ruby: &str) -> CharState {
        CharState {
            unit: crate::evaluate_chars::TextUnit::Ruby {
                base: vec![aviutl2_text_parser::Element::Text(base.to_string())],
                ruby: vec![aviutl2_text_parser::Element::Text(ruby.to_string())],
                scale: None,
                expand_line_height: false,
            },
            control_index: 0,
            start_time: 0.0,
        }
    }

    fn states_to_text(states: &[CharState]) -> String {
        states.iter().map(|state| state.unit.to_string()).collect()
    }

    #[test]
    fn test_segment_with_budoux() {
        let char_states = "私は学生です。"
            .chars()
            .map(make_char_state)
            .collect::<Vec<_>>();
        let segments = segment_with_budoux(&char_states);
        let texts = segments
            .iter()
            .map(|s| states_to_text(&s.chars))
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["私は", "学生です。"]);
    }

    #[test]
    fn test_segment_with_budoux_keeps_ruby_atomic() {
        let char_states = vec![
            make_char_state('私'),
            make_char_state('は'),
            make_ruby_state("学生", "がくせい"),
            make_char_state('で'),
            make_char_state('す'),
        ];
        let segments = segment_with_budoux(&char_states);
        let flattened = segments
            .iter()
            .flat_map(|segment| segment.chars.iter())
            .collect::<Vec<_>>();
        assert_eq!(flattened, char_states.iter().collect::<Vec<_>>());
        assert_eq!(
            flattened
                .iter()
                .filter(|state| {
                    matches!(state.unit, crate::evaluate_chars::TextUnit::Ruby { .. })
                })
                .count(),
            1
        );
    }

    #[test]
    fn test_segment_with_whitespace() {
        let char_states = "hello world"
            .chars()
            .map(make_char_state)
            .collect::<Vec<_>>();
        let segments = segment_with_whitespace(&char_states);
        let texts = segments
            .iter()
            .map(|s| states_to_text(&s.chars))
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["hello", "world"]);
    }

    #[test]
    fn test_segment_with_whitespace_double_spaces() {
        let char_states = "hello  world"
            .chars()
            .map(make_char_state)
            .collect::<Vec<_>>();
        let segments = segment_with_whitespace(&char_states);
        let texts = segments
            .iter()
            .map(|s| states_to_text(&s.chars))
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["hello", "world"]);
        assert_eq!(segments.len(), 2);
        assert!(matches!(segments[0].wrapped_by, WrappedBy::None));
        assert!(matches!(segments[1].wrapped_by, WrappedBy::Whitespace(_)));
    }

    #[test]
    fn test_segment_manually() {
        let char_states = "私は\\b学生です。"
            .chars()
            .map(make_char_state)
            .collect::<Vec<_>>();
        let segments = segment_manually(&char_states);
        let texts = segments
            .iter()
            .map(|s| states_to_text(&s.chars))
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["私は", "学生です。"]);
        assert!(matches!(segments[0].wrapped_by, WrappedBy::None));
        assert!(matches!(segments[1].wrapped_by, WrappedBy::Manual));
    }

    #[test]
    fn test_segment_with_whitespace_does_not_use_budoux() {
        let char_states = "私は学生です。"
            .chars()
            .map(make_char_state)
            .collect::<Vec<_>>();
        let segments = segment_with_whitespace(&char_states);
        let texts = segments
            .iter()
            .map(|s| states_to_text(&s.chars))
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["私は学生です。"]);
        assert!(matches!(segments[0].wrapped_by, WrappedBy::None));
    }
}
