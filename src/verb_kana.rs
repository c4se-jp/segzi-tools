use std::collections::BTreeMap;
use std::ops::Range;

#[derive(Debug, Clone, Copy)]
enum Rule {
    GodanHa,
    Ichidan {
        modern: char,
        historical: &'static str,
    },
}

#[derive(Debug)]
struct Entry {
    reading: String,
    conjugation: String,
    modern_base: String,
    prefix: String,
    rule: Rule,
}

#[derive(Debug)]
pub(crate) struct VerbKana {
    entries: BTreeMap<String, Vec<Entry>>,
}

impl VerbKana {
    pub(crate) fn parse(input: &str) -> Result<Self, String> {
        let mut entries: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
        for (line_number, line) in input.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let error = || format!("invalid verb kana dictionary row {}", line_number + 1);
            let fields: Vec<_> = line.split('\t').collect();
            let [lexeme, reading, conjugation, modern, historical, source] = fields.as_slice()
            else {
                return Err(error());
            };
            if [lexeme, reading, conjugation, modern, historical, source]
                .iter()
                .any(|field| field.is_empty())
            {
                return Err(error());
            }
            let (modern_suffix, historical_suffix, rule) = match *conjugation {
                "五段-ワア行" => ("う", "ふ", Rule::GodanHa),
                "下一段-ア行" if historical.ends_with("へる") => (
                    "える",
                    "へる",
                    Rule::Ichidan {
                        modern: 'え',
                        historical: "へ",
                    },
                ),
                "下一段-ア行" => (
                    "える",
                    "ゑる",
                    Rule::Ichidan {
                        modern: 'え',
                        historical: "ゑ",
                    },
                ),
                "上一段-ア行" => (
                    "いる",
                    "ゐる",
                    Rule::Ichidan {
                        modern: 'い',
                        historical: "ゐ",
                    },
                ),
                _ => return Err(error()),
            };
            let prefix = modern.strip_suffix(modern_suffix).ok_or_else(error)?;
            // Only a checked kanji spelling is eligible; do not disambiguate
            // kana-only homophones such as いる by the best analysis alone.
            if !prefix.chars().any(|c| {
                matches!(
                    c,
                    '\u{3400}'..='\u{4dbf}'
                        | '\u{4e00}'..='\u{9fff}'
                        | '\u{f900}'..='\u{faff}'
                        | '\u{20000}'..='\u{323af}'
                )
            }) || historical.strip_suffix(historical_suffix) != Some(prefix)
            {
                return Err(error());
            }
            let variants = entries.entry((*lexeme).to_owned()).or_default();
            if variants
                .iter()
                .any(|entry| entry.reading == *reading && entry.modern_base == *modern)
            {
                return Err(error());
            }
            variants.push(Entry {
                reading: (*reading).to_owned(),
                conjugation: (*conjugation).to_owned(),
                modern_base: (*modern).to_owned(),
                prefix: prefix.to_owned(),
                rule,
            });
        }
        Ok(Self { entries })
    }

    pub(crate) fn replacement(
        &self,
        surface: &str,
        details: &[&str],
    ) -> Option<(Range<usize>, &'static str)> {
        // Embedded UniDic: POS 0, cType 4, cForm 5, lForm 6,
        // lemma 7, orthBase 10. Unknown words have fewer details.
        if details.len() < 11 || details[0] != "動詞" {
            return None;
        }
        for entry in self.entries.get(details[7])? {
            if details[6] != entry.reading {
                continue;
            }
            let Some(tail) = surface.strip_prefix(&entry.prefix) else {
                continue;
            };
            if details[4] == entry.conjugation && details[10] == entry.modern_base {
                match entry.rule {
                    Rule::GodanHa => {
                        let historical = match (details[5], tail) {
                            ("終止形-一般" | "連体形-一般", "う") => "ふ",
                            ("未然形-一般", "わ") => "は",
                            ("連用形-一般", "い") => "ひ",
                            ("仮定形-一般" | "命令形", "え") => "へ",
                            ("意志推量形", "おう") => "はう",
                            // Keep sokuonbin and u-onbin, e.g. 思った / 問うた.
                            _ => continue,
                        };
                        return Some((entry.prefix.len()..surface.len(), historical));
                    }
                    Rule::Ichidan { modern, historical } => {
                        if let Some(edit) =
                            ichidan_edit(entry.prefix.len(), tail, details[5], modern, historical)
                        {
                            return Some(edit);
                        }
                    }
                }
            }
            // UniDic gives potential forms such as 思える the lemma / lForm
            // 思う / オモウ, but a different cType and orthBase.
            if matches!(entry.rule, Rule::GodanHa)
                && details[4] == "下一段-ア行"
                && details[10].strip_prefix(&entry.prefix) == Some("える")
                && let Some(edit) = ichidan_edit(entry.prefix.len(), tail, details[5], 'え', "へ")
            {
                return Some(edit);
            }
        }
        None
    }
}

fn ichidan_edit(
    prefix_len: usize,
    tail: &str,
    form: &str,
    modern: char,
    historical: &'static str,
) -> Option<(Range<usize>, &'static str)> {
    let ending = tail.strip_prefix(modern)?;
    match (form, ending) {
        ("終止形-一般" | "連体形-一般", "る")
        | ("未然形-一般" | "連用形-一般", "")
        | ("仮定形-一般", "れ")
        | ("命令形", "ろ" | "よ")
        | ("意志推量形", "よう") => {
            Some((prefix_len..prefix_len + modern.len_utf8(), historical))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{VerbKana, ichidan_edit};

    #[test]
    fn matches_ichidan_endings_without_reconstructing_the_word() {
        for (form, ending) in [
            ("終止形-一般", "る"),
            ("連体形-一般", "る"),
            ("未然形-一般", ""),
            ("連用形-一般", ""),
            ("仮定形-一般", "れ"),
            ("命令形", "ろ"),
            ("命令形", "よ"),
            ("意志推量形", "よう"),
        ] {
            for (modern, historical) in [('え', "へ"), ('え', "ゑ"), ('い', "ゐ")] {
                let tail = format!("{modern}{ending}");
                assert_eq!(
                    ichidan_edit(3, &tail, form, modern, historical),
                    Some((3..6, historical))
                );
            }
        }
        assert!(ichidan_edit(3, "へる", "終止形-一般", 'え', "へ").is_none());
        assert!(ichidan_edit(3, "えった", "連用形-促音便", 'え', "へ").is_none());
    }

    #[test]
    fn rejects_inconsistent_or_ambiguous_dictionary_rows() {
        let valid = "思う\tオモウ\t五段-ワア行\t思う\t思ふ\tsource\n";
        assert!(VerbKana::parse(valid).is_ok());
        for invalid in [
            "思う\tオモウ\t五段-ワア行\t思う\t思ふ\n",
            "思う\tオモウ\t五段-ワア行\t思う\t思ふ\t\n",
            "思う\tオモウ\t五段-ワア行\t思う\t違ふ\tsource\n",
            "思う\tオモウ\tunknown\t思う\t思ふ\tsource\n",
            "いる\tイル\t上一段-ア行\tいる\tゐる\tsource\n",
            "いる\tイル\t上一段-ア行\tイいる\tイゐる\tsource\n",
            "植える\tウエル\t下一段-ア行\t植える\t植へるる\tsource\n",
            &format!("{valid}{valid}"),
        ] {
            assert!(VerbKana::parse(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn requires_every_unidic_identity_field_and_a_supported_form() {
        let rules = VerbKana::parse("思う\tオモウ\t五段-ワア行\t思う\t思ふ\tsource\n").unwrap();
        let details = [
            "動詞",
            "一般",
            "*",
            "*",
            "五段-ワア行",
            "終止形-一般",
            "オモウ",
            "思う",
            "思う",
            "オモウ",
            "思う",
        ];
        assert_eq!(rules.replacement("思う", &details), Some((3..6, "ふ")));
        for index in [0, 4, 5, 6, 7, 10] {
            let mut wrong = details;
            wrong[index] = "*";
            assert!(rules.replacement("思う", &wrong).is_none(), "{index}");
        }
        assert!(rules.replacement("おもう", &details).is_none());
        assert!(rules.replacement("思ふ", &details).is_none());
        assert!(rules.replacement("思う", &["UNK"]).is_none());
    }
}
