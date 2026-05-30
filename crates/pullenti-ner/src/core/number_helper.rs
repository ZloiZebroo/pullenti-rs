use crate::source_of_analysis::SourceOfAnalysis;
use crate::token::{TokenKind, TokenRef};

#[derive(Clone)]
pub struct NumberParseResult {
    pub value: String,
    pub end: TokenRef,
    pub is_ordinal: bool,
}

#[derive(Clone)]
pub struct NumberRangeParseResult {
    pub values: Vec<String>,
    pub end: TokenRef,
}

pub fn try_parse_number(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<NumberParseResult> {
    if let Some(res) = try_parse_digit_number(t, sofa) {
        return Some(res);
    }
    try_parse_word_number(t, sofa)
}

pub fn try_parse_number_range(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<NumberRangeParseResult> {
    let first = try_parse_number(t, sofa)?;
    let Some(sep) = first.end.borrow().next.clone() else {
        return Some(NumberRangeParseResult { values: vec![first.value], end: first.end });
    };
    if !sep.borrow().is_hiphen(sofa) {
        return Some(NumberRangeParseResult { values: vec![first.value], end: first.end });
    }
    let Some(second_start) = sep.borrow().next.clone() else {
        return Some(NumberRangeParseResult { values: vec![first.value], end: first.end });
    };
    let Some(second) = try_parse_number(&second_start, sofa) else {
        return Some(NumberRangeParseResult { values: vec![first.value], end: first.end });
    };
    Some(NumberRangeParseResult {
        values: vec![first.value, second.value],
        end: second.end,
    })
}

fn try_parse_digit_number(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<NumberParseResult> {
    let int_str = match &t.borrow().kind {
        TokenKind::Number(n) => n.value.clone(),
        _ => return None,
    };

    let next = t.borrow().next.clone();
    if let Some(ref sep) = next {
        let sep_b = sep.borrow();
        if sep_b.whitespaces_before_count(sofa) == 0 && sep_b.length_char() == 1 {
            let sep_ch = sofa.char_at(sep_b.begin_char);
            if sep_ch == ',' || sep_ch == '.' {
                let after_sep = sep_b.next.clone();
                drop(sep_b);
                if let Some(ref frac_tok) = after_sep {
                    let fb = frac_tok.borrow();
                    if fb.whitespaces_before_count(sofa) == 0 {
                        if let TokenKind::Number(n) = &fb.kind {
                            return Some(NumberParseResult {
                                value: format!("{}.{}", int_str, n.value),
                                end: frac_tok.clone(),
                                is_ordinal: false,
                            });
                        }
                    }
                }
            }
        }
    }

    Some(NumberParseResult { value: int_str, end: t.clone(), is_ordinal: false })
}

fn try_parse_word_number(t: &TokenRef, sofa: &SourceOfAnalysis) -> Option<NumberParseResult> {
    let mut total: i64 = 0;
    let mut current: i64 = 0;
    let mut end: Option<TokenRef> = None;
    let mut cur = Some(t.clone());
    let mut saw = false;
    let mut is_ordinal = false;

    while let Some(tok) = cur.clone() {
        if saw && tok.borrow().whitespaces_before_count(sofa) > 1 {
            break;
        }
        let Some(word) = number_word_value(&tok) else {
            break;
        };
        saw = true;
        is_ordinal = word.ordinal;
        match word.kind {
            NumberWordKind::Value(v) => current += v,
            NumberWordKind::Multiplier(m) => {
                if current == 0 {
                    current = 1;
                }
                total += current * m;
                current = 0;
            }
        }
        end = Some(tok.clone());
        cur = tok.borrow().next.clone();
    }

    if !saw {
        return None;
    }
    let value = total + current;
    Some(NumberParseResult {
        value: value.to_string(),
        end: end.unwrap(),
        is_ordinal,
    })
}

struct NumberWord {
    kind: NumberWordKind,
    ordinal: bool,
}

enum NumberWordKind {
    Value(i64),
    Multiplier(i64),
}

fn number_word_value(t: &TokenRef) -> Option<NumberWord> {
    let tb = t.borrow();
    let TokenKind::Text(txt) = &tb.kind else { return None; };
    if let Some(w) = number_word_by_term(&txt.term) {
        return Some(w);
    }
    for wf in tb.morph.items() {
        if let Some(s) = wf.normal_case.as_deref() {
            if let Some(w) = number_word_by_term(s) {
                return Some(w);
            }
        }
        if let Some(s) = wf.normal_full.as_deref() {
            if let Some(w) = number_word_by_term(s) {
                return Some(w);
            }
        }
    }
    None
}

fn number_word_by_term(term: &str) -> Option<NumberWord> {
    let (value, ordinal) = match term {
        "НОЛЬ" | "НУЛЬ" => (0, false),
        "ОДИН" | "ОДНА" | "ОДНО" | "ОДНОГО" | "ОДНОЙ" | "ОДНУ" | "ПЕРВЫЙ" | "ПЕРВАЯ" | "ПЕРВОЕ" | "ПЕРВОГО" => (1, term.starts_with("ПЕРВ")),
        "ДВА" | "ДВЕ" | "ДВУХ" | "ДВУМ" | "ВТОРОЙ" | "ВТОРАЯ" | "ВТОРОЕ" | "ВТОРОГО" => (2, term.starts_with("ВТОР")),
        "ТРИ" | "ТРЕХ" | "ТРЁХ" | "ТРЕМ" | "ТРЁМ" | "ТРЕТИЙ" | "ТРЕТЬЯ" | "ТРЕТЬЕ" | "ТРЕТЬЕГО" => (3, term.starts_with("ТРЕТ")),
        "ЧЕТЫРЕ" | "ЧЕТЫРЕХ" | "ЧЕТЫРЁХ" | "ЧЕТВЕРТЫЙ" | "ЧЕТВЁРТЫЙ" | "ЧЕТВЕРТОГО" | "ЧЕТВЁРТОГО" => (4, term.starts_with("ЧЕТВЕР") || term.starts_with("ЧЕТВЁР")),
        "ПЯТЬ" | "ПЯТИ" | "ПЯТЫЙ" | "ПЯТОГО" => (5, term.starts_with("ПЯТ")),
        "ШЕСТЬ" | "ШЕСТИ" | "ШЕСТОЙ" | "ШЕСТОГО" => (6, term.starts_with("ШЕСТО")),
        "СЕМЬ" | "СЕМИ" | "СЕДЬМОЙ" | "СЕДЬМОГО" => (7, term.starts_with("СЕДЬМ")),
        "ВОСЕМЬ" | "ВОСЬМИ" | "ВОСЬМОЙ" | "ВОСЬМОГО" => (8, term.starts_with("ВОСЬМ")),
        "ДЕВЯТЬ" | "ДЕВЯТИ" | "ДЕВЯТЫЙ" | "ДЕВЯТОГО" => (9, term.starts_with("ДЕВЯТ")),
        "ДЕСЯТЬ" | "ДЕСЯТИ" | "ДЕСЯТЫЙ" | "ДЕСЯТОГО" => (10, term.starts_with("ДЕСЯТ")),
        "ОДИННАДЦАТЬ" | "ОДИННАДЦАТЫЙ" | "ОДИННАДЦАТОГО" => (11, term.starts_with("ОДИННАДЦАТ") && term != "ОДИННАДЦАТЬ"),
        "ДВЕНАДЦАТЬ" | "ДВЕНАДЦАТЫЙ" | "ДВЕНАДЦАТОГО" => (12, term.starts_with("ДВЕНАДЦАТ") && term != "ДВЕНАДЦАТЬ"),
        "ТРИНАДЦАТЬ" | "ТРИНАДЦАТЫЙ" | "ТРИНАДЦАТОГО" => (13, term.starts_with("ТРИНАДЦАТ") && term != "ТРИНАДЦАТЬ"),
        "ЧЕТЫРНАДЦАТЬ" | "ЧЕТЫРНАДЦАТЫЙ" | "ЧЕТЫРНАДЦАТОГО" => (14, term.starts_with("ЧЕТЫРНАДЦАТ") && term != "ЧЕТЫРНАДЦАТЬ"),
        "ПЯТНАДЦАТЬ" | "ПЯТНАДЦАТЫЙ" | "ПЯТНАДЦАТОГО" => (15, term.starts_with("ПЯТНАДЦАТ") && term != "ПЯТНАДЦАТЬ"),
        "ШЕСТНАДЦАТЬ" | "ШЕСТНАДЦАТЫЙ" | "ШЕСТНАДЦАТОГО" => (16, term.starts_with("ШЕСТНАДЦАТ") && term != "ШЕСТНАДЦАТЬ"),
        "СЕМНАДЦАТЬ" | "СЕМНАДЦАТЫЙ" | "СЕМНАДЦАТОГО" => (17, term.starts_with("СЕМНАДЦАТ") && term != "СЕМНАДЦАТЬ"),
        "ВОСЕМНАДЦАТЬ" | "ВОСЕМНАДЦАТЫЙ" | "ВОСЕМНАДЦАТОГО" => (18, term.starts_with("ВОСЕМНАДЦАТ") && term != "ВОСЕМНАДЦАТЬ"),
        "ДЕВЯТНАДЦАТЬ" | "ДЕВЯТНАДЦАТЫЙ" | "ДЕВЯТНАДЦАТОГО" => (19, term.starts_with("ДЕВЯТНАДЦАТ") && term != "ДЕВЯТНАДЦАТЬ"),
        "ДВАДЦАТЬ" | "ДВАДЦАТЫЙ" | "ДВАДЦАТОГО" => (20, term.starts_with("ДВАДЦАТ") && term != "ДВАДЦАТЬ"),
        "ТРИДЦАТЬ" | "ТРИДЦАТЫЙ" | "ТРИДЦАТОГО" => (30, term.starts_with("ТРИДЦАТ") && term != "ТРИДЦАТЬ"),
        "СОРОК" | "СОРОКОВОЙ" | "СОРОКОВОГО" => (40, term.starts_with("СОРОКОВ")),
        "ПЯТЬДЕСЯТ" | "ПЯТИДЕСЯТЫЙ" | "ПЯТИДЕСЯТОГО" => (50, term.starts_with("ПЯТИДЕСЯТ")),
        "ШЕСТЬДЕСЯТ" | "ШЕСТИДЕСЯТЫЙ" | "ШЕСТИДЕСЯТОГО" => (60, term.starts_with("ШЕСТИДЕСЯТ")),
        "СЕМЬДЕСЯТ" | "СЕМИДЕСЯТЫЙ" | "СЕМИДЕСЯТОГО" => (70, term.starts_with("СЕМИДЕСЯТ")),
        "ВОСЕМЬДЕСЯТ" | "ВОСЬМИДЕСЯТЫЙ" | "ВОСЬМИДЕСЯТОГО" => (80, term.starts_with("ВОСЬМИДЕСЯТ")),
        "ДЕВЯНОСТО" | "ДЕВЯНОСТЫЙ" | "ДЕВЯНОСТОГО" => (90, term.starts_with("ДЕВЯНОСТ") && term != "ДЕВЯНОСТО"),
        "СТО" | "СОТЫЙ" | "СОТОГО" => (100, term.starts_with("СОТ")),
        "ДВЕСТИ" | "ДВУХСОТЫЙ" | "ДВУХСОТОГО" => (200, term.starts_with("ДВУХСОТ")),
        "ТРИСТА" | "ТРЕХСОТЫЙ" | "ТРЁХСОТЫЙ" | "ТРЕХСОТОГО" | "ТРЁХСОТОГО" => (300, term.starts_with("ТРЕХСОТ") || term.starts_with("ТРЁХСОТ")),
        "ЧЕТЫРЕСТА" | "ЧЕТЫРЕХСОТЫЙ" | "ЧЕТЫРЁХСОТЫЙ" | "ЧЕТЫРЕХСОТОГО" | "ЧЕТЫРЁХСОТОГО" => (400, term.starts_with("ЧЕТЫРЕХСОТ") || term.starts_with("ЧЕТЫРЁХСОТ")),
        "ПЯТЬСОТ" | "ШЕСТЬСОТ" | "СЕМЬСОТ" | "ВОСЕМЬСОТ" | "ДЕВЯТЬСОТ" => {
            let v = match term {
                "ПЯТЬСОТ" => 500,
                "ШЕСТЬСОТ" => 600,
                "СЕМЬСОТ" => 700,
                "ВОСЕМЬСОТ" => 800,
                _ => 900,
            };
            (v, false)
        }
        "ТЫСЯЧА" | "ТЫСЯЧИ" | "ТЫСЯЧ" => return Some(NumberWord { kind: NumberWordKind::Multiplier(1000), ordinal: false }),
        "МИЛЛИОН" | "МИЛЛИОНА" | "МИЛЛИОНОВ" => return Some(NumberWord { kind: NumberWordKind::Multiplier(1_000_000), ordinal: false }),
        _ => return None,
    };
    Some(NumberWord { kind: NumberWordKind::Value(value), ordinal })
}
