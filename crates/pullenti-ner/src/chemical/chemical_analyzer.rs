/// ChemicalAnalyzer — mirrors `ChemicalAnalyzer.cs`.
///
/// Recognizes chemical formulas (H2O, CO2, NaCl) and named substances (вода, кислота).

use std::rc::Rc;
use std::cell::RefCell;

use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::token::{Token, TokenRef};
use super::chemical_token::{try_parse_list, create_referent};

pub struct ChemicalAnalyzer;

impl ChemicalAnalyzer {
    pub fn new() -> Self { ChemicalAnalyzer }
}

impl Analyzer for ChemicalAnalyzer {
    fn name(&self) -> &'static str { "CHEMICAL" }
    fn caption(&self) -> &'static str { "Химические формулы" }

    fn process(&self, kit: &mut AnalysisKit) {
        let sofa = kit.sofa.clone();
        let mut probs: Vec<(TokenRef, TokenRef)> = Vec::new(); // (begin, end) for doubtful spans
        let mut cur = kit.first_token.clone();

        while let Some(t) = cur.clone() {
            if t.borrow().is_ignored(&sofa) {
                cur = t.borrow().next.clone();
                continue;
            }

            let li = match try_parse_list(&t, &sofa, 0) {
                None => { cur = t.borrow().next.clone(); continue; }
                Some(v) if v.is_empty() => { cur = t.borrow().next.clone(); continue; }
                Some(v) => v,
            };

            let last_end = li.last().unwrap().end_token.clone();
            let referent_opt = create_referent(&li, &sofa);

            match referent_opt {
                None => {
                    // Save as probable (needs context) — process after main pass
                    probs.push((t.clone(), last_end.clone()));
                    cur = last_end.borrow().next.clone();
                }
                Some(referent) => {
                    let r_rc = Rc::new(RefCell::new(referent));
                    let r_rc = kit.add_entity(r_rc);
                    let tok = Rc::new(RefCell::new(
                        Token::new_referent(t.clone(), last_end.clone(), r_rc)
                    ));
                    kit.embed_token(tok.clone());
                    cur = tok.borrow().next.clone();
                }
            }
        }

        // Second pass: try to create referents for probable spans that may now have context
        for (begin, end) in probs {
            // Re-parse from begin
            if let Some(li) = try_parse_list(&begin, &sofa, 0) {
                if !li.is_empty() {
                    if let Some(referent) = create_referent(&li, &sofa) {
                        let r_rc = Rc::new(RefCell::new(referent));
                        let r_rc = kit.add_entity(r_rc);
                        let tok = Rc::new(RefCell::new(
                            Token::new_referent(begin.clone(), end.clone(), r_rc)
                        ));
                        kit.embed_token(tok);
                    }
                }
            }
        }
    }
}
