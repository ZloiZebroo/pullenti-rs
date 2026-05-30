pub mod mail_analyzer;
pub mod mail_line;
pub mod mail_referent;

pub use mail_analyzer::MailAnalyzer;
pub use mail_referent::{
    get_kind, get_text, new_mail_referent, set_kind, set_text, MailKind, ATTR_KIND, ATTR_REF,
    ATTR_TEXT, OBJ_TYPENAME as MAIL_OBJ_TYPENAME,
};
