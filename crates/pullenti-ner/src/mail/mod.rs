pub mod mail_referent;
pub mod mail_line;
pub mod mail_analyzer;

pub use mail_analyzer::MailAnalyzer;
pub use mail_referent::{
    OBJ_TYPENAME as MAIL_OBJ_TYPENAME,
    ATTR_KIND, ATTR_TEXT, ATTR_REF,
    MailKind,
    get_kind, get_text, set_kind, set_text,
    new_mail_referent,
};
