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
        if char_states[i].char == '\\'
            && i + 1 < char_states.len()
            && char_states[i + 1].char == 'b'
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
    let text: String = char_states.iter().map(|c| c.char).collect();
    let text_segments = crate::budoux::segment(&text);
    let mut result = Vec::new();
    let mut index = 0;

    for text_segment in text_segments {
        let len = text_segment.chars().count();
        result.push(Segment {
            chars: char_states[index..index + len].to_vec(),
            wrapped_by: if index == 0 {
                WrappedBy::None
            } else {
                WrappedBy::Budoux
            },
        });
        index += len;
    }

    result
}

pub fn segment_with_whitespace(char_states: &[CharState]) -> Vec<Segment> {
    let mut result = Vec::new();
    let mut run_start: Option<usize> = None;
    let mut pending_whitespace: Vec<CharState> = Vec::new();

    for (i, char_state) in char_states.iter().enumerate() {
        if char_state.char.is_whitespace() {
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
            char: c,
            bold: false,
            italic: false,
            strikethrough: false,
            size: 12.0,
            color: "FFFFFF".to_string(),
            font: "Arial".to_string(),
            start_time: 0.0,
            secondary_color: "000000".to_string(),
            outline_size: 0.0,
        }
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
            .map(|s| s.chars.iter().map(|c| c.char).collect::<String>())
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["私は", "学生です。"]);
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
            .map(|s| s.chars.iter().map(|c| c.char).collect::<String>())
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
            .map(|s| s.chars.iter().map(|c| c.char).collect::<String>())
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
            .map(|s| s.chars.iter().map(|c| c.char).collect::<String>())
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
            .map(|s| s.chars.iter().map(|c| c.char).collect::<String>())
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["私は学生です。"]);
        assert!(matches!(segments[0].wrapped_by, WrappedBy::None));
    }
}
