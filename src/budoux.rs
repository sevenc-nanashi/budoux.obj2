// Based on [google/budoux: parser.py](https://github.com/google/budoux/blob/main/budoux/parser.py),
// Licensed under the Apache License.
//
// https://github.com/google/budoux/blob/main/LICENSE

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
struct BudouxModel {
    #[serde(default)]
    uw1: std::collections::HashMap<String, i32>,
    #[serde(default)]
    uw2: std::collections::HashMap<String, i32>,
    #[serde(default)]
    uw3: std::collections::HashMap<String, i32>,
    #[serde(default)]
    uw4: std::collections::HashMap<String, i32>,
    #[serde(default)]
    uw5: std::collections::HashMap<String, i32>,
    #[serde(default)]
    uw6: std::collections::HashMap<String, i32>,
    #[serde(default)]
    bw1: std::collections::HashMap<String, i32>,
    #[serde(default)]
    bw2: std::collections::HashMap<String, i32>,
    #[serde(default)]
    bw3: std::collections::HashMap<String, i32>,
    #[serde(default)]
    tw1: std::collections::HashMap<String, i32>,
    #[serde(default)]
    tw2: std::collections::HashMap<String, i32>,
    #[serde(default)]
    tw3: std::collections::HashMap<String, i32>,
    #[serde(default)]
    tw4: std::collections::HashMap<String, i32>,
}

static MODEL: std::sync::LazyLock<BudouxModel> = std::sync::LazyLock::new(|| {
    let value =
        include_json::include_json!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/model/ja.json"));
    serde_json::from_value(value).expect("Bundled model is invalid")
});
static BASE_VALUE: std::sync::LazyLock<i32> = std::sync::LazyLock::new(|| {
    let mut sum = 0;
    for node in [
        &MODEL.uw1, &MODEL.uw2, &MODEL.uw3, &MODEL.uw4, &MODEL.uw5, &MODEL.uw6, &MODEL.bw1,
        &MODEL.bw2, &MODEL.bw3, &MODEL.tw1, &MODEL.tw2, &MODEL.tw3, &MODEL.tw4,
    ] {
        for score in node.values() {
            sum += score;
        }
    }

    -sum / 2
});

pub fn segment(text: &str) -> Vec<String> {
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut result = vec![chars[0].to_string()];
    for i in 1..chars.len() {
        let mut score = *BASE_VALUE;
        if i > 2 {
            score += MODEL.uw1.get(&chars[i - 3].to_string()).unwrap_or(&0);
        }
        if i > 1 {
            score += MODEL.uw2.get(&chars[i - 2].to_string()).unwrap_or(&0);
        }
        score += MODEL.uw3.get(&chars[i - 1].to_string()).unwrap_or(&0);
        score += MODEL.uw4.get(&chars[i].to_string()).unwrap_or(&0);
        if i < chars.len() - 1 {
            score += MODEL.uw5.get(&chars[i + 1].to_string()).unwrap_or(&0);
        }
        if i < chars.len() - 2 {
            score += MODEL.uw6.get(&chars[i + 2].to_string()).unwrap_or(&0);
        }

        if i > 1 {
            let key = format!("{}{}", chars[i - 2], chars[i - 1]);
            score += MODEL.bw1.get(&key).unwrap_or(&0);
        }
        let key = format!("{}{}", chars[i - 1], chars[i]);
        score += MODEL.bw2.get(&key).unwrap_or(&0);
        if i < chars.len() - 1 {
            let key = format!("{}{}", chars[i], chars[i + 1]);
            score += MODEL.bw3.get(&key).unwrap_or(&0);
        }

        if i > 2 {
            let key = format!("{}{}{}", chars[i - 3], chars[i - 2], chars[i - 1]);
            score += MODEL.tw1.get(&key).unwrap_or(&0);
        }
        if i > 1 && i < chars.len() - 1 {
            let key = format!("{}{}{}", chars[i - 2], chars[i - 1], chars[i]);
            score += MODEL.tw2.get(&key).unwrap_or(&0);
        }
        if i > 0 && i < chars.len() - 2 {
            let key = format!("{}{}{}", chars[i - 1], chars[i], chars[i + 1]);
            score += MODEL.tw3.get(&key).unwrap_or(&0);
        }
        if i < chars.len() - 3 {
            let key = format!("{}{}{}", chars[i], chars[i + 1], chars[i + 2]);
            score += MODEL.tw4.get(&key).unwrap_or(&0);
        }

        if score > 0 {
            result.push(current);
            current = String::new();
        }
        current.push(chars[i]);
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

pub fn segment_char_states(
    char_states: &[crate::evaluate_chars::CharState],
) -> Vec<Vec<crate::evaluate_chars::CharState>> {
    let mut result = Vec::new();
    let mut run_start = 0;

    for (i, char_state) in char_states.iter().enumerate() {
        if !char_state.char.is_whitespace() {
            continue;
        }

        if run_start < i {
            let text: String = char_states[run_start..i].iter().map(|c| c.char).collect();
            let segments = segment(&text);
            let mut index = run_start;
            for segment in segments {
                let segment_len = segment.chars().count();
                result.push(char_states[index..index + segment_len].to_vec());
                index += segment_len;
            }
        }

        result.push(vec![char_state.clone()]);
        run_start = i + 1;
    }

    if run_start < char_states.len() {
        let text: String = char_states[run_start..].iter().map(|c| c.char).collect();
        let segments = segment(&text);
        let mut index = run_start;
        for segment in segments {
            let segment_len = segment.chars().count();
            result.push(char_states[index..index + segment_len].to_vec());
            index += segment_len;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluate_chars::CharState;

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
            end_time: None,
        }
    }

    #[test]
    fn test_segment() {
        let text = "私は学生です。";
        let segments = segment(text);
        assert_eq!(segments, vec!["私は", "学生です。"]);
    }

    #[test]
    fn test_segment_char_states_split_whitespace() {
        let char_states = "hello world"
            .chars()
            .map(make_char_state)
            .collect::<Vec<_>>();
        let segments = segment_char_states(&char_states);
        let segment_texts = segments
            .iter()
            .map(|segment| segment.iter().map(|c| c.char).collect::<String>())
            .collect::<Vec<_>>();
        assert_eq!(segment_texts, vec!["hello", " ", "world"]);
    }
}
