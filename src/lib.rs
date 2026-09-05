use regex::Regex;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct SegziMap {
    char_map: BTreeMap<String, String>,
    ambiguous_characters: BTreeMap<String, Vec<Candidate>>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbiguousCharacter {
    pub character: String,
    pub count: usize,
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    pub unresolved_ambiguous_characters: Vec<AmbiguousCharacter>,
}

pub struct Converter {
    zh_compounds: Vec<(String, String)>,
    zh_chars: BTreeMap<char, String>,
    chars: BTreeMap<char, String>,
    kana: Vec<(String, String)>,
    patterns: Vec<(Regex, String)>,
    ambiguous: BTreeMap<char, Vec<String>>,
}

fn rows(input: &str) -> Vec<(String, String)> {
    input
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let mut fields = line.split('\t');
            Some((fields.next()?.to_owned(), fields.next()?.to_owned()))
        })
        .collect()
}

fn replacement_rows(input: &str) -> Vec<(String, String)> {
    let mut result = rows(input);
    result.sort_by_key(|(from, _)| std::cmp::Reverse(from.chars().count()));
    result
}

fn char_map(rows: Vec<(String, String)>) -> BTreeMap<char, String> {
    rows.into_iter()
        .filter_map(|(from, to)| {
            (from.chars().count() == 1).then(|| (from.chars().next().unwrap(), to))
        })
        .collect()
}

impl Converter {
    pub fn embedded() -> Result<Self, String> {
        let segzi: SegziMap = serde_json::from_str(include_str!("../dic/kyuji_map.json"))
            .map_err(|error| error.to_string())?;
        let mut ambiguous = BTreeMap::new();
        for (source, candidates) in segzi.ambiguous_characters {
            if let Some(character) = source.chars().next() {
                ambiguous.insert(
                    character,
                    candidates
                        .into_iter()
                        .map(|candidate| candidate.target.unwrap_or(source.clone()))
                        .collect(),
                );
            }
        }
        let zh_ambiguous: BTreeMap<String, Vec<Candidate>> =
            serde_json::from_str(include_str!("../dic/zh_ambiguous_characters.json"))
                .map_err(|error| error.to_string())?;
        for (source, candidates) in zh_ambiguous {
            if let Some(character) = source.chars().next() {
                ambiguous.insert(
                    character,
                    candidates
                        .into_iter()
                        .map(|candidate| candidate.target.unwrap_or(source.clone()))
                        .collect(),
                );
            }
        }
        Ok(Self {
            zh_compounds: replacement_rows(include_str!("../dic/zh_compound_map.tsv")),
            zh_chars: char_map(rows(include_str!("../dic/zh_char_map.tsv"))),
            chars: char_map(segzi.char_map.into_iter().collect()),
            kana: rows(include_str!("../dic/kana_replacements.tsv")),
            patterns: rows(include_str!("../dic/kana_patterns.tsv"))
                .into_iter()
                .map(|(pattern, replacement)| {
                    Regex::new(&pattern)
                        .map(|regex| (regex, replacement))
                        .map_err(|e| e.to_string())
                })
                .collect::<Result<_, _>>()?,
            ambiguous,
        })
    }

    pub fn convert(&self, source: &str) -> (String, Report) {
        let mut text = source.to_owned();
        for (from, to) in &self.zh_compounds {
            text = text.replace(from, to);
        }
        text = translate(&text, &self.zh_chars);
        text = translate(&text, &self.chars);
        for (from, to) in &self.kana {
            text = text.replace(from, to);
        }
        for (pattern, replacement) in &self.patterns {
            text = pattern.replace_all(&text, replacement).into_owned();
        }
        let mut unresolved_ambiguous_characters: Vec<_> = self
            .ambiguous
            .iter()
            .filter_map(|(character, candidates)| {
                let count = text.chars().filter(|c| c == character).count();
                (count > 0).then(|| AmbiguousCharacter {
                    character: character.to_string(),
                    count,
                    candidates: candidates.clone(),
                })
            })
            .collect();
        unresolved_ambiguous_characters
            .sort_by(|a, b| b.count.cmp(&a.count).then(a.character.cmp(&b.character)));
        (
            text,
            Report {
                unresolved_ambiguous_characters,
            },
        )
    }
}

fn translate(text: &str, map: &BTreeMap<char, String>) -> String {
    text.chars()
        .map(|character| {
            map.get(&character)
                .cloned()
                .unwrap_or_else(|| character.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Converter;

    #[test]
    fn converts_embedded_non_boundary_stages() {
        let converter = Converter::embedded().unwrap();
        let (text, report) =
            converter.convert("きょうという日は待っているようです。删除制造干后。");
        assert_eq!(text, "けふといふ日は待ってゐるやうです。削除製造干后。");
        assert_eq!(report.unresolved_ambiguous_characters.len(), 2);
    }

    #[test]
    fn reports_ambiguity_created_by_chinese_character_conversion() {
        let converter = Converter::embedded().unwrap();
        let (text, report) = converter.convert("证");
        assert_eq!(text, "証");
        assert!(
            report
                .unresolved_ambiguous_characters
                .iter()
                .any(|item| item.character == "証" && item.count == 1)
        );
    }
}
