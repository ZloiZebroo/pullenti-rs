pub mod currency_table;
pub mod money_analyzer;
pub mod money_referent;

pub use money_analyzer::MoneyAnalyzer;
pub use money_referent::{
    get_currency, get_rest, get_value, new_money_referent, real_value, set_currency, set_rest,
    set_value, value_f64, ATTR_ALTREST, ATTR_ALTVALUE, ATTR_CURRENCY, ATTR_REST, ATTR_VALUE,
    OBJ_TYPENAME as MONEY_OBJ_TYPENAME,
};
