/// MailAnalyzer — segments an e-mail text into Head / Hello / Body / Tail blocks.
///
/// Simplified port of `Pullenti.Ner.Mail.MailAnalyzer`.
/// This is a *specific* analyzer (is_specific = true).

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::token::Token;

use super::mail_referent::{self as mr, MailKind};
use super::mail_line::{self, MailLine, MailLineType};

pub struct MailAnalyzer;

impl MailAnalyzer {
    pub fn new() -> Self { MailAnalyzer }
}

impl Default for MailAnalyzer {
    fn default() -> Self { MailAnalyzer }
}

impl Analyzer for MailAnalyzer {
    fn name(&self) -> &'static str { "MAIL" }
    fn caption(&self) -> &'static str { "Блок письма" }
    fn is_specific(&self) -> bool { true }
    fn progress_weight(&self) -> i32 { 1 }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();

        // ── Step 1: collect MailLines ────────────────────────────────────
        let mut lines: Vec<MailLine> = Vec::new();
        {
            let first = match kit.first_token.clone() { Some(t) => t, None => return };

            // First token: always emit a line regardless of newline
            let mut cur = Some(first);
            let mut is_first_token = true;

            while let Some(tok) = cur.clone() {
                let is_nl = {
                    let tb = tok.borrow();
                    tb.is_newline_before(&sofa)
                };

                if is_first_token || is_nl {
                    is_first_token = false;
                    if let Some(ml) = mail_line::parse(&tok, &sofa) {
                        let end_tok = ml.end_token.clone();
                        lines.push(ml);
                        // Advance past the line's end
                        cur = end_tok.borrow().next.clone();
                        continue;
                    }
                }
                cur = tok.borrow().next.clone();
            }
        }

        if lines.is_empty() { return; }

        // ── Step 2: group lines into blocks ──────────────────────────────
        // A new block is started when:
        //   - we encounter a From-type line that looks like a new forwarded section
        // All non-From lines after the last From block belong to one main block.
        let n = lines.len();
        let mut blocks: Vec<Vec<usize>> = Vec::new(); // indices into `lines`
        let mut blk: Option<Vec<usize>> = None;
        let mut i = 0usize;

        while i < n {
            let typ_i = lines[i].typ;

            if typ_i == MailLineType::From {
                let mut is_new = lines[i].must_be_first_line || i == 0;

                // If next two lines include From or Hello → new block
                if !is_new && (i + 2) < n {
                    let t1 = lines[i + 1].typ;
                    let t2 = lines[i + 2].typ;
                    if t1 == MailLineType::From || t2 == MailLineType::From
                        || t1 == MailLineType::Hello || t2 == MailLineType::Hello
                    {
                        is_new = true;
                    }
                }

                // Check if preceded by BestRegards
                if !is_new {
                    let mut j = i as i32 - 1;
                    while j >= 0 {
                        if lines[j as usize].typ != MailLineType::Undefined {
                            if lines[j as usize].typ == MailLineType::BestRegards {
                                is_new = true;
                            }
                            break;
                        }
                        j -= 1;
                    }
                }

                // Check if line contains DATE or URI referent
                if !is_new {
                    let end_char = lines[i].end_char();
                    let mut t = Some(lines[i].begin_token.clone());
                    while let Some(tok) = t.clone() {
                        if tok.borrow().end_char > end_char { break; }
                        if let Some(r) = tok.borrow().get_referent() {
                            let rtype = r.borrow().type_name.clone();
                            if rtype == "DATE" || rtype == "URI" {
                                is_new = true;
                                break;
                            }
                        }
                        t = tok.borrow().next.clone();
                    }
                }

                if is_new {
                    // Collect consecutive From lines into a new block
                    blocks.push(Vec::new());
                    let blk_idx = blocks.len() - 1;

                    while i < n {
                        if lines[i].typ == MailLineType::From {
                            if !blocks[blk_idx].is_empty() && lines[i].must_be_first_line {
                                break;
                            }
                            blocks[blk_idx].push(i);
                        } else if i + 1 < n && lines[i + 1].typ == MailLineType::From {
                            // Lookahead: check if existing block already has a real From
                            let blk_has_real = blocks[blk_idx].iter().any(|&bi| {
                                let l = &lines[bi];
                                l.is_real_from() || l.must_be_first_line || l.mail_addr().is_some()
                            });
                            if !blk_has_real {
                                blocks[blk_idx].push(i);
                                i += 1;
                                continue;
                            }
                            // Check if upcoming From lines are "real"
                            let upcoming_real = {
                                let mut found = false;
                                let mut j = i + 1;
                                while j < n && lines[j].typ == MailLineType::From {
                                    if lines[j].is_real_from()
                                        || lines[j].must_be_first_line
                                        || lines[j].mail_addr().is_some()
                                    {
                                        found = true;
                                        break;
                                    }
                                    j += 1;
                                }
                                found
                            };
                            if upcoming_real {
                                break;
                            }
                            blocks[blk_idx].push(i);
                        } else {
                            break;
                        }
                        i += 1;
                    }
                    i = i.saturating_sub(1);
                    i += 1;
                    continue;
                }
            }

            // Non-From line (or From that didn't start a new block)
            if blk.is_none() {
                blocks.push(Vec::new());
                blk = Some(Vec::new());
            }
            if let Some(ref mut b) = blk {
                b.push(i);
            }
            // Also push to last block
            let last = blocks.last_mut().unwrap();
            if last.last() != Some(&i) {
                last.push(i);
            }
            i += 1;
        }

        // Handle remaining blk (if non-From lines were collected separately)
        if let Some(b) = blk {
            if !b.is_empty() && blocks.last().map_or(true, |lb| lb.last() != b.last()) {
                blocks.push(b);
            }
        }

        if blocks.is_empty() { return; }

        // ── Step 3: create MailReferent entities for each block ──────────
        for blk_indices in &blocks {
            if blk_indices.is_empty() { continue; }
            let blk_lines: Vec<&MailLine> = blk_indices.iter().map(|&i| &lines[i]).collect();
            process_block(blk_lines, &sofa, kit);
        }
    }
}

// ── Block processing ──────────────────────────────────────────────────────

fn process_block(
    lines: Vec<&MailLine>,
    sofa: &crate::source_of_analysis::SourceOfAnalysis,
    kit: &mut AnalysisKit,
) {
    if lines.is_empty() { return; }
    let n = lines.len();
    let mut i0 = 0usize;

    // Head block: leading From lines
    if lines[0].typ == MailLineType::From {
        let mut head_end_idx = 0usize;
        let mut ii = 0usize;
        while ii < n {
            if lines[ii].typ == MailLineType::From {
                head_end_idx = ii;
            } else if ii + 1 < n && lines[ii + 1].typ == MailLineType::From {
                // allow one non-From line before a From line
            } else {
                break;
            }
            ii += 1;
        }
        i0 = head_end_idx + 1;

        let begin_tok = lines[0].begin_token.clone();
        let end_tok   = lines[head_end_idx].end_token.clone();
        let text = sofa.substring(begin_tok.borrow().begin_char, end_tok.borrow().end_char).to_string();

        let mut r = mr::new_mail_referent();
        mr::set_kind(&mut r, MailKind::Head);
        mr::set_text(&mut r, &text);
        let r_rc = Rc::new(RefCell::new(r));
        kit.add_entity(r_rc.clone());
        let tok = Rc::new(RefCell::new(Token::new_referent(begin_tok, end_tok, r_rc)));
        kit.embed_token(tok);
    }

    // Find the tail start (BestRegards or person-only lines at the end)
    let mut tail_start: Option<usize> = None; // index into lines[]

    // Scan backwards for BestRegards
    {
        let mut ii = n as i32 - 1;
        while ii >= i0 as i32 {
            let li = &lines[ii as usize];
            if li.typ == MailLineType::BestRegards {
                let mut t2_idx = ii as usize;
                ii -= 1;
                while ii >= i0 as i32 {
                    let prev = &lines[ii as usize];
                    if prev.typ == MailLineType::BestRegards && prev.words(sofa) < 2 {
                        t2_idx = ii as usize;
                    } else if ii > i0 as i32
                        && prev.words(sofa) < 3
                        && lines[ii as usize - 1].typ == MailLineType::BestRegards
                        && lines[ii as usize - 1].words(sofa) < 2
                    {
                        ii -= 1;
                        t2_idx = ii as usize;
                    } else {
                        break;
                    }
                    ii -= 1;
                }
                tail_start = Some(t2_idx);
                break;
            }

            // Short line with refs
            if li.refs.len() > 0 && li.words(sofa) < 3 && (ii as usize) > i0 {
                tail_start = Some(ii as usize);
                ii -= 1;
                continue;
            }

            if li.words(sofa) > 10 {
                tail_start = None;
            } else if li.words(sofa) > 2 {
                // Some tolerance
            }
            ii -= 1;
        }
    }

    // Fallback: look for person-only line at end
    if tail_start.is_none() {
        let mut ii = n as i32 - 1;
        while ii >= i0 as i32 {
            let li = &lines[ii as usize];
            if li.typ == MailLineType::Undefined {
                if li.refs.len() > 0
                    && li.refs.iter().any(|r| r.borrow().type_name == "PERSON")
                    && li.words(sofa) == 0
                    && (ii as usize) > i0
                {
                    tail_start = Some(ii as usize);
                    break;
                }
            }
            ii -= 1;
        }
    }

    // Hello block: search for Hello line from i0
    {
        let mut ii = i0;
        while ii < n {
            if lines[ii].typ == MailLineType::Hello {
                if i0 <= ii {
                    let begin_tok = lines[i0].begin_token.clone();
                    let end_tok   = lines[ii].end_token.clone();
                    if begin_tok.borrow().begin_char <= end_tok.borrow().end_char {
                        let text = sofa.substring(
                            begin_tok.borrow().begin_char,
                            end_tok.borrow().end_char,
                        ).to_string();
                        let mut r = mr::new_mail_referent();
                        mr::set_kind(&mut r, MailKind::Hello);
                        mr::set_text(&mut r, &text);
                        let r_rc = Rc::new(RefCell::new(r));
                        kit.add_entity(r_rc.clone());
                        let tok = Rc::new(RefCell::new(Token::new_referent(begin_tok, end_tok, r_rc)));
                        kit.embed_token(tok);
                        i0 = ii + 1;
                    }
                }
                break;
            } else if lines[ii].typ != MailLineType::Undefined
                || lines[ii].words(sofa) > 0
                || !lines[ii].refs.is_empty()
            {
                break;
            }
            ii += 1;
        }
    }

    if i0 >= n { return; }

    // Body block
    {
        let body_end_idx = match tail_start {
            Some(ts) if ts > i0 => {
                // end_token of line before tail_start
                // We want body = [i0 .. ts-1]
                Some(ts - 1)
            }
            Some(ts) if ts == i0 => None, // tail starts right where body would
            _ => Some(n - 1),
        };

        if let Some(end_idx) = body_end_idx {
            if end_idx >= i0 {
                let begin_tok = lines[i0].begin_token.clone();
                let end_tok   = lines[end_idx].end_token.clone();
                if begin_tok.borrow().begin_char <= end_tok.borrow().end_char {
                    let text = sofa.substring(
                        begin_tok.borrow().begin_char,
                        end_tok.borrow().end_char,
                    ).to_string();
                    let mut r = mr::new_mail_referent();
                    mr::set_kind(&mut r, MailKind::Body);
                    mr::set_text(&mut r, &text);
                    let r_rc = Rc::new(RefCell::new(r));
                    kit.add_entity(r_rc.clone());
                    let tok = Rc::new(RefCell::new(Token::new_referent(begin_tok, end_tok, r_rc)));
                    kit.embed_token(tok);
                }
            }
        }
    }

    // Tail block
    if let Some(ts) = tail_start {
        if ts < n {
            let begin_tok = lines[ts].begin_token.clone();
            let end_tok   = lines[n - 1].end_token.clone();
            if begin_tok.borrow().begin_char <= end_tok.borrow().end_char {
                let text = sofa.substring(
                    begin_tok.borrow().begin_char,
                    end_tok.borrow().end_char,
                ).to_string();
                let mut r = mr::new_mail_referent();
                mr::set_kind(&mut r, MailKind::Tail);
                mr::set_text(&mut r, &text);
                let r_rc = Rc::new(RefCell::new(r));
                kit.add_entity(r_rc.clone());
                let tok = Rc::new(RefCell::new(Token::new_referent(begin_tok, end_tok, r_rc)));
                kit.embed_token(tok);
            }
        }
    }
}
