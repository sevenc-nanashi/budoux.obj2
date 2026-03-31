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
        .map(|(key, value)| {
            let mut chars = key.chars();
            let c1 = chars.next().expect("trigram key is empty");
            let c2 = chars.next().expect("trigram key must have 3 chars");
            let c3 = chars.next().expect("trigram key must have 3 chars");
            assert!(
                chars.next().is_none(),
                "trigram key must have exactly three chars: {key}",
            );
            ((c1, c2, c3), value)
        })
        .collect()
}

static MODEL: std::sync::LazyLock<BudouxModel> = std::sync::LazyLock::new(|| {
    let value =
        include_json::include_json!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/model/ja.json"));
    let raw: BudouxModelRaw = serde_json::from_value(value).expect("Bundled model is invalid");
    BudouxModel::from_raw(raw)
});
static BASE_VALUE: std::sync::LazyLock<i32> = std::sync::LazyLock::new(|| {
    let mut sum = 0;
    for node in [
        &MODEL.uw1,
        &MODEL.uw2,
        &MODEL.uw3,
        &MODEL.uw4,
        &MODEL.uw5,
        &MODEL.uw6,
    ] {
        for score in node.values() {
            sum += score;
        }
    }
    for node in [&MODEL.bw1, &MODEL.bw2, &MODEL.bw3] {
        for score in node.values() {
            sum += score;
        }
    }
    for node in [&MODEL.tw1, &MODEL.tw2, &MODEL.tw3, &MODEL.tw4] {
        for score in node.values() {
            sum += score;
        }
    }

    -sum / 2
});

pub fn segment(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return vec![];
    }

    let mut result = vec![chars[0].to_string()];

    for i in 1..chars.len() {
        let mut score = *BASE_VALUE;

        if i > 2 {
            score += MODEL.uw1.get(&chars[i - 3]).unwrap_or(&0);
        }
        if i > 1 {
            score += MODEL.uw2.get(&chars[i - 2]).unwrap_or(&0);
        }
        score += MODEL.uw3.get(&chars[i - 1]).unwrap_or(&0);
        score += MODEL.uw4.get(&chars[i]).unwrap_or(&0);

        if i + 1 < chars.len() {
            score += MODEL.uw5.get(&chars[i + 1]).unwrap_or(&0);
        }
        if i + 2 < chars.len() {
            score += MODEL.uw6.get(&chars[i + 2]).unwrap_or(&0);
        }

        if i > 1 {
            score += MODEL.bw1.get(&(chars[i - 2], chars[i - 1])).unwrap_or(&0);
        }
        {
            score += MODEL.bw2.get(&(chars[i - 1], chars[i])).unwrap_or(&0);
        }
        if i + 1 < chars.len() {
            score += MODEL.bw3.get(&(chars[i], chars[i + 1])).unwrap_or(&0);
        }

        if i > 2 {
            score += MODEL
                .tw1
                .get(&(chars[i - 3], chars[i - 2], chars[i - 1]))
                .unwrap_or(&0);
        }
        if i > 1 {
            score += MODEL
                .tw2
                .get(&(chars[i - 2], chars[i - 1], chars[i]))
                .unwrap_or(&0);
        }
        if i + 1 < chars.len() {
            score += MODEL
                .tw3
                .get(&(chars[i - 1], chars[i], chars[i + 1]))
                .unwrap_or(&0);
        }
        if i + 2 < chars.len() {
            score += MODEL
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
    fn test_segment() {
        let text = "私は学生です。";
        let segments = segment(text);
        assert_eq!(segments, vec!["私は", "学生です。"]);
    }
}
