pub mod money_referent;
pub mod currency_table;
pub mod money_analyzer;

pub use money_analyzer::MoneyAnalyzer;
pub use money_referent::{
    OBJ_TYPENAME as MONEY_OBJ_TYPENAME,
    ATTR_CURRENCY, ATTR_VALUE, ATTR_REST, ATTR_ALTVALUE, ATTR_ALTREST,
    get_currency, get_value, get_rest, value_f64, real_value,
    set_currency, set_value, set_rest,
    new_money_referent,
};
