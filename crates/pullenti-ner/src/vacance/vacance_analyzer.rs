/// VacanceAnalyzer — job vacancy text parser.
/// Mirrors `VacanceAnalyzer.cs`.

use std::rc::Rc;
use std::cell::RefCell;
use crate::analyzer::Analyzer;
use crate::analysis_kit::AnalysisKit;
use crate::token::Token;
use super::vacance_referent::{
    OBJ_TYPENAME as _, new_vacancy_referent, set_item_type, set_value, set_expired,
    VacanceItemType, ATTR_REF,
};
use super::vacance_token::{VacanceToken, VacanceTokenType};

pub struct VacanceAnalyzer;

impl VacanceAnalyzer {
    pub fn new() -> Self { VacanceAnalyzer }
}

impl Default for VacanceAnalyzer {
    fn default() -> Self { VacanceAnalyzer }
}

impl Analyzer for VacanceAnalyzer {
    fn name(&self) -> &'static str { "VACANCE" }
    fn caption(&self) -> &'static str { "Вакансия" }
    fn is_specific(&self) -> bool { true }

    fn process(&self, kit: &mut AnalysisKit) {
        let first = match kit.first_token.clone() {
            Some(t) => t,
            None    => return,
        };
        let sofa = &kit.sofa.clone();

        let li = match VacanceToken::try_parse_list(&first, sofa) {
            Some(v) if !v.is_empty() => v,
            _ => return,
        };

        // Check if vacancy is expired
        let is_expired = li.iter().any(|v| v.typ == VacanceTokenType::Expired);

        for v in li {
            // Skip items with no content
            if v.value.is_none() && v.refs.is_empty() { continue; }

            let item_type = match v.typ {
                VacanceTokenType::Date       => VacanceItemType::Date,
                VacanceTokenType::Experience => VacanceItemType::Experience,
                VacanceTokenType::Money      => VacanceItemType::Money,
                VacanceTokenType::Name       => VacanceItemType::Name,
                VacanceTokenType::Education  => VacanceItemType::Education,
                VacanceTokenType::Language   => VacanceItemType::Language,
                VacanceTokenType::Driving    => VacanceItemType::DrivingLicense,
                VacanceTokenType::License    => VacanceItemType::License,
                VacanceTokenType::Moral      => VacanceItemType::Moral,
                VacanceTokenType::Plus       => VacanceItemType::Plus,
                VacanceTokenType::Skill      => VacanceItemType::Skill,
                _ => continue,
            };

            let mut referent = new_vacancy_referent();
            set_item_type(&mut referent, item_type);

            if item_type == VacanceItemType::Name && is_expired {
                set_expired(&mut referent);
            }

            if let Some(val) = &v.value {
                set_value(&mut referent, val);
            }

            // Add referent-valued slots for each collected ref
            for r in &v.refs {
                referent.add_slot(
                    ATTR_REF,
                    crate::referent::SlotValue::Referent(r.clone()),
                    false,
                );
            }

            let r_rc = Rc::new(RefCell::new(referent));
            let r_rc = kit.add_entity(r_rc);
            let tok = Rc::new(RefCell::new(
                Token::new_referent(v.begin_token.clone(), v.end_token.clone(), r_rc)
            ));
            kit.embed_token(tok);
        }
    }
}
