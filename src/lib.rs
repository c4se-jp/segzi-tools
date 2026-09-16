use lindera::{dictionary::load_dictionary, mode::Mode, segmenter::Segmenter};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AmbiguousCharacter {
    pub character: String,
    pub count: usize,
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Report {
    pub unresolved_ambiguous_characters: Vec<AmbiguousCharacter>,
    pub boundary_skipped_compound_replacements: Vec<CompoundReplacement>,
    pub unresolved_bunka_replacements: Vec<CompoundReplacement>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompoundReplacement {
    pub source: String,
    pub target: String,
    pub count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BunkaReplacementKind {
    Safe,
    Candidate,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BunkaReplacement {
    source: String,
    target: String,
    kind: BunkaReplacementKind,
}

pub struct Converter {
    zh_compounds: Vec<(String, String)>,
    compounds: Vec<(String, String)>,
    bunka_candidates: Vec<BunkaReplacement>,
    zh_chars: BTreeMap<char, String>,
    chars: BTreeMap<char, String>,
    segmentation_chars: BTreeMap<char, char>,
    kana: Vec<(String, String)>,
    patterns: Vec<(Regex, String)>,
    ambiguous: BTreeMap<char, Vec<String>>,
    segmenter: Segmenter,
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

fn bunka_replacement_rows(input: &str) -> Result<Vec<BunkaReplacement>, String> {
    let mut replacements = Vec::new();
    for (line_number, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let source = fields
            .next()
            .ok_or_else(|| format!("文化廳置換表 {} 行目にsourceがありません", line_number + 1))?;
        let target = fields
            .next()
            .ok_or_else(|| format!("文化廳置換表 {} 行目にtargetがありません", line_number + 1))?;
        let kind = match fields.next() {
            Some("safe") => BunkaReplacementKind::Safe,
            Some("candidate") => BunkaReplacementKind::Candidate,
            Some("pending") => BunkaReplacementKind::Pending,
            Some(kind) => {
                return Err(format!(
                    "文化廳置換表 {} 行目のkindが不正です: {kind}",
                    line_number + 1
                ));
            }
            None => {
                return Err(format!(
                    "文化廳置換表 {} 行目にkindがありません",
                    line_number + 1
                ));
            }
        };
        replacements.push(BunkaReplacement {
            source: source.to_owned(),
            target: target.to_owned(),
            kind,
        });
    }
    replacements.sort_by_key(|replacement| std::cmp::Reverse(replacement.source.chars().count()));
    Ok(replacements)
}

fn char_map(rows: Vec<(String, String)>) -> BTreeMap<char, String> {
    rows.into_iter()
        .filter_map(|(from, to)| {
            (from.chars().count() == 1).then(|| (from.chars().next().unwrap(), to))
        })
        .collect()
}

fn segmentation_char_map(rows: Vec<(String, String)>) -> BTreeMap<char, char> {
    rows.into_iter()
        .filter_map(|(from, to)| {
            (from.chars().count() == 1 && to.chars().count() == 1)
                .then(|| (from.chars().next().unwrap(), to.chars().next().unwrap()))
        })
        .collect()
}

impl Converter {
    pub fn embedded() -> Result<Self, String> {
        let segzi: SegziMap = serde_json::from_str(include_str!("../dic/kiuzi_map.json"))
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
        let chars = char_map(segzi.char_map.into_iter().collect());
        let bunka_replacements =
            bunka_replacement_rows(include_str!("../dic/compound_replacements_bunka.tsv"))?;
        let (safe_bunka_replacements, bunka_candidates): (Vec<_>, Vec<_>) = bunka_replacements
            .into_iter()
            .partition(|replacement| replacement.kind == BunkaReplacementKind::Safe);
        Ok(Self {
            zh_compounds: replacement_rows(include_str!("../dic/zh_compound_map.tsv")),
            compounds: {
                let mut rows = replacement_rows(include_str!("../dic/compound_replacements.tsv"));
                rows.extend(
                    safe_bunka_replacements.iter().map(|replacement| {
                        (replacement.source.clone(), replacement.target.clone())
                    }),
                );
                rows
            },
            bunka_candidates,
            zh_chars: char_map(rows(include_str!("../dic/zh_char_map.tsv"))),
            segmentation_chars: segmentation_char_map(rows(include_str!(
                "../dic/unidic_normalization.tsv"
            ))),
            chars,
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
            segmenter: Segmenter::new(
                Mode::Normal,
                load_dictionary("embedded://unidic").map_err(|e| e.to_string())?,
                None,
            ),
        })
    }

    pub fn convert(&self, source: &str) -> (String, Report) {
        let mut text = source.to_owned();
        for (from, to) in &self.zh_compounds {
            text = text.replace(from, to);
        }
        text = translate(&text, &self.zh_chars);
        let unresolved_bunka_replacements = self.bunka_replacement_report(&text);
        let mut skipped = BTreeMap::new();
        for (source, target) in &self.compounds {
            let mut start = text.find(source);
            if start.is_none() {
                continue;
            }
            let boundaries = self.boundaries(&text);
            let mut pieces = Vec::new();
            let mut last = 0;
            while let Some(found) = start {
                let end = found + source.len();
                if boundaries.contains(&found) && boundaries.contains(&end) {
                    pieces.push(&text[last..found]);
                    pieces.push(target);
                    last = end;
                } else {
                    *skipped.entry((source.clone(), target.clone())).or_insert(0) += 1;
                }
                start = text[end..].find(source).map(|next| end + next);
            }
            if last != 0 {
                pieces.push(&text[last..]);
                text = pieces.concat();
            }
        }
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
                boundary_skipped_compound_replacements: skipped
                    .into_iter()
                    .map(|((source, target), count)| CompoundReplacement {
                        source,
                        target,
                        count,
                    })
                    .collect(),
                unresolved_bunka_replacements,
            },
        )
    }

    fn bunka_replacement_report(&self, text: &str) -> Vec<CompoundReplacement> {
        let boundaries = self.boundaries(text);
        let mut unresolved = BTreeMap::new();
        for replacement in &self.bunka_candidates {
            let mut start = text.find(&replacement.source);
            while let Some(found) = start {
                let end = found + replacement.source.len();
                if boundaries.contains(&found) && boundaries.contains(&end) {
                    *unresolved
                        .entry((replacement.source.clone(), replacement.target.clone()))
                        .or_insert(0) += 1;
                }
                start = text[end..].find(&replacement.source).map(|next| end + next);
            }
        }
        unresolved
            .into_iter()
            .map(|((source, target), count)| CompoundReplacement {
                source,
                target,
                count,
            })
            .collect()
    }

    fn boundaries(&self, text: &str) -> std::collections::BTreeSet<usize> {
        let mut boundaries: std::collections::BTreeSet<usize> =
            [0, text.len()].into_iter().collect();
        let mut segmentation_text = String::with_capacity(text.len());
        let mut source_offsets = BTreeMap::from([(0, 0)]);
        let mut source_offset = 0;
        for character in text.chars() {
            segmentation_text.push(
                self.segmentation_chars
                    .get(&character)
                    .copied()
                    .unwrap_or(character),
            );
            source_offset += character.len_utf8();
            source_offsets.insert(segmentation_text.len(), source_offset);
        }
        if let Ok(tokens) = self.segmenter.segment(Cow::Borrowed(&segmentation_text)) {
            for token in tokens {
                if let Some(source_offset) = source_offsets.get(&token.byte_end) {
                    boundaries.insert(*source_offset);
                }
            }
        }
        boundaries
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

    #[test]
    fn does_not_report_pending_bunka_replacements_inside_another_word() {
        let converter = Converter::embedded().unwrap();
        let (text, report) = converter.convert("提案分布。提案分布");
        assert_eq!(text, "提案分布。提案分布");
        assert!(report.unresolved_bunka_replacements.is_empty());
    }

    #[test]
    fn reports_pending_bunka_replacements_at_word_boundaries() {
        let converter = Converter::embedded().unwrap();
        let (text, report) = converter.convert("案分をする。提案分布。");
        assert_eq!(text, "案分をする。提案分布。");
        assert!(
            report
                .unresolved_bunka_replacements
                .iter()
                .any(|item| item.source == "案分" && item.target == "按分" && item.count == 1)
        );
    }

    #[test]
    fn converts_compounds_at_boundaries_after_old_character_normalization() {
        let converter = Converter::embedded().unwrap();
        assert_eq!(converter.segmentation_chars.get(&'檢'), Some(&'検'));
        assert_eq!(converter.segmentation_chars.get(&'實'), Some(&'実'));
        assert!(!converter.segmentation_chars.contains_key(&'決'));
        assert!(!converter.segmentation_chars.contains_key(&'京'));
        let (text, report) = converter.convert(include_str!(
            "../docs/adr-0002-github-actions-workflow-design.md"
        ));
        assert!(text.contains("檢證"));
        assert!(text.contains("初囘實行"));
        assert!(text.contains("餘裕"));
        assert!(
            !report
                .boundary_skipped_compound_replacements
                .iter()
                .any(|item| ["檢証", "初回", "余裕"].contains(&item.source.as_str()))
        );
    }

    #[test]
    fn reports_pending_bunka_replacements_without_changing_the_input() {
        let converter = Converter::embedded().unwrap();
        let (text, report) = converter.convert("膨大な資料");
        assert_eq!(text, "膨大な資料");
        assert!(
            report
                .unresolved_bunka_replacements
                .iter()
                .any(|item| item.source == "膨大" && item.target == "厖大" && item.count == 1)
        );
    }
}
