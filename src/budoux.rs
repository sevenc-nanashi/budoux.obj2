// Based on [google/budoux: parser.py](https://github.com/google/budoux/blob/main/budoux/parser.py),
// Licensed under the Apache License.
//
// https://github.com/google/budoux/blob/main/LICENSE

type UniGram = std::collections::HashMap<char, i32>;
type BiGram = std::collections::HashMap<(char, char), i32>;
type TriGram = std::collections::HashMap<(char, char, char), i32>;

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
struct BudouxModel {
    #[serde(default)]
    uw1: UniGram,
    #[serde(default)]
    uw2: UniGram,
    #[serde(default)]
    uw3: UniGram,
    #[serde(default)]
    uw4: UniGram,
    #[serde(default)]
    uw5: UniGram,
    #[serde(default)]
    uw6: UniGram,
    #[serde(default)]
    bw1: BiGram,
    #[serde(default)]
    bw2: BiGram,
    #[serde(default)]
    bw3: BiGram,
    #[serde(default)]
    tw1: TriGram,
    #[serde(default)]
    tw2: TriGram,
    #[serde(default)]
    tw3: TriGram,
    #[serde(default)]
    tw4: TriGram,
}

impl BudouxModel {
    fn from_raw(raw: BudouxModelRaw) -> Self {
        Self {
            uw1: parse_unigram(raw.uw1),
            uw2: parse_unigram(raw.uw2),
            uw3: parse_unigram(raw.uw3),
            uw4: parse_unigram(raw.uw4),
            uw5: parse_unigram(raw.uw5),
            uw6: parse_unigram(raw.uw6),
            bw1: parse_bigram(raw.bw1),
            bw2: parse_bigram(raw.bw2),
            bw3: parse_bigram(raw.bw3),
            tw1: parse_trigram(raw.tw1),
            tw2: parse_trigram(raw.tw2),
            tw3: parse_trigram(raw.tw3),
            tw4: parse_trigram(raw.tw4),
        }
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
struct BudouxModelRaw {
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

fn parse_unigram(input: std::collections::HashMap<String, i32>) -> UniGram {
    input
        .into_iter()
        .map(|(key, value)| {
            let mut chars = key.chars();
            let c1 = chars.next().expect("unigram key is empty");
            assert!(
                chars.next().is_none(),
                "unigram key must have exactly one char: {key}",
            );
            (c1, value)
        })
        .collect()
}

fn parse_bigram(input: std::collections::HashMap<String, i32>) -> BiGram {
    input
        .into_iter()
        .map(|(key, value)| {
            let mut chars = key.chars();
            let c1 = chars.next().expect("bigram key is empty");
            let c2 = chars.next().expect("bigram key must have 2 chars");
            assert!(
                chars.next().is_none(),
                "bigram key must have exactly two chars: {key}",
            );
            ((c1, c2), value)
        })
        .collect()
}

fn parse_trigram(input: std::collections::HashMap<String, i32>) -> TriGram {
    input
        .into_iter()
        .filter_map(|(key, value)| {
            let mut chars = key.chars();
            let c1 = chars.next().expect("trigram key is empty");
            let c2 = chars.next().expect("trigram key must have 3 chars");
            let c3 = chars.next()?;
            chars.next().is_none().then_some(((c1, c2, c3), value))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Japanese,
    SimplifiedChinese,
    TraditionalChinese,
    Thai,
}

struct ParserModel {
    model: BudouxModel,
    base_value: i32,
}

fn load_model(value: serde_json::Value) -> ParserModel {
    let raw: BudouxModelRaw = serde_json::from_value(value).expect("Bundled model is invalid");
    let sum = [
        &raw.uw1, &raw.uw2, &raw.uw3, &raw.uw4, &raw.uw5, &raw.uw6, &raw.bw1, &raw.bw2, &raw.bw3,
        &raw.tw1, &raw.tw2, &raw.tw3, &raw.tw4,
    ]
    .into_iter()
    .flat_map(|node| node.values())
    .sum::<i32>();
    let model = BudouxModel::from_raw(raw);

    ParserModel {
        model,
        base_value: -sum / 2,
    }
}

macro_rules! bundled_model {
    ($name:ident, $file:literal) => {
        static $name: std::sync::LazyLock<ParserModel> = std::sync::LazyLock::new(|| {
            load_model(include_json::include_json!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                $file
            )))
        });
    };
}

bundled_model!(JAPANESE_MODEL, "/src/model/ja.json");
bundled_model!(SIMPLIFIED_CHINESE_MODEL, "/src/model/zh-hans.json");
bundled_model!(TRADITIONAL_CHINESE_MODEL, "/src/model/zh-hant.json");
bundled_model!(THAI_MODEL, "/src/model/th.json");

fn get_model(language: Language) -> &'static ParserModel {
    match language {
        Language::Japanese => &JAPANESE_MODEL,
        Language::SimplifiedChinese => &SIMPLIFIED_CHINESE_MODEL,
        Language::TraditionalChinese => &TRADITIONAL_CHINESE_MODEL,
        Language::Thai => &THAI_MODEL,
    }
}

pub fn segment(text: &str, language: Language) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return vec![];
    }

    let parser = get_model(language);
    let model = &parser.model;
    let mut result = vec![chars[0].to_string()];

    for i in 1..chars.len() {
        let mut score = parser.base_value;

        if i > 2 {
            score += model.uw1.get(&chars[i - 3]).unwrap_or(&0);
        }
        if i > 1 {
            score += model.uw2.get(&chars[i - 2]).unwrap_or(&0);
        }
        score += model.uw3.get(&chars[i - 1]).unwrap_or(&0);
        score += model.uw4.get(&chars[i]).unwrap_or(&0);

        if i + 1 < chars.len() {
            score += model.uw5.get(&chars[i + 1]).unwrap_or(&0);
        }
        if i + 2 < chars.len() {
            score += model.uw6.get(&chars[i + 2]).unwrap_or(&0);
        }

        if i > 1 {
            score += model.bw1.get(&(chars[i - 2], chars[i - 1])).unwrap_or(&0);
        }
        {
            score += model.bw2.get(&(chars[i - 1], chars[i])).unwrap_or(&0);
        }
        if i + 1 < chars.len() {
            score += model.bw3.get(&(chars[i], chars[i + 1])).unwrap_or(&0);
        }

        if i > 2 {
            score += model
                .tw1
                .get(&(chars[i - 3], chars[i - 2], chars[i - 1]))
                .unwrap_or(&0);
        }
        if i > 1 {
            score += model
                .tw2
                .get(&(chars[i - 2], chars[i - 1], chars[i]))
                .unwrap_or(&0);
        }
        if i + 1 < chars.len() {
            score += model
                .tw3
                .get(&(chars[i - 1], chars[i], chars[i + 1]))
                .unwrap_or(&0);
        }
        if i + 2 < chars.len() {
            score += model
                .tw4
                .get(&(chars[i], chars[i + 1], chars[i + 2]))
                .unwrap_or(&0);
        }

        if score > 0 {
            result.push(chars[i].to_string());
        } else {
            result.last_mut().unwrap().push(chars[i]);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_japanese() {
        assert_eq!(
            segment("私は学生です。", Language::Japanese),
            vec!["私は", "学生です。"]
        );
    }

    #[test]
    fn test_segment_simplified_chinese() {
        assert_eq!(
            segment("今天是晴天。", Language::SimplifiedChinese),
            vec!["今天", "是", "晴天。"]
        );
    }

    #[test]
    fn test_segment_traditional_chinese() {
        assert_eq!(
            segment("今天是晴天。", Language::TraditionalChinese),
            vec!["今天", "是", "晴天。"]
        );
    }

    #[test]
    fn test_segment_thai() {
        assert_eq!(
            segment("วันนี้อากาศดี", Language::Thai),
            vec!["วัน", "นี้", "อากาศ", "ดี"]
        );
    }
}
