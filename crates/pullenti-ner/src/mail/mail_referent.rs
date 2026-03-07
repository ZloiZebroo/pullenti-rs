/// MailKind — type of email block.
///
/// Ported from Pullenti C# `MailKind.cs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MailKind {
    #[default]
    Undefined = 0,
    /// Header block (From/To/Date/Subject lines)
    Head = 1,
    /// Greeting / salutation
    Hello = 2,
    /// Message body
    Body = 3,
    /// Sign-off / signature
    Tail = 4,
}

impl MailKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MailKind::Undefined => "UNDEFINED",
            MailKind::Head      => "HEAD",
            MailKind::Hello     => "HELLO",
            MailKind::Body      => "BODY",
            MailKind::Tail      => "TAIL",
        }
    }
}

// ── Referent type / attribute name constants ───────────────────────────────

/// Entity type name — "MAIL"
pub const OBJ_TYPENAME: &str = "MAIL";
/// Attribute: block type (MailKind as uppercase string)
pub const ATTR_KIND: &str = "TYPE";
/// Attribute: plain text of the block
pub const ATTR_TEXT: &str = "TEXT";
/// Attribute: reference to another entity found inside the block
pub const ATTR_REF: &str = "REF";

// ── Accessor helpers ───────────────────────────────────────────────────────

use crate::referent::{Referent, SlotValue};

/// Create a new MAIL Referent.
pub fn new_mail_referent() -> Referent {
    Referent::new(OBJ_TYPENAME)
}

/// Set the KIND slot (clear existing).
pub fn set_kind(r: &mut Referent, kind: MailKind) {
    r.add_slot(ATTR_KIND, SlotValue::Str(kind.as_str().to_string()), true);
}

/// Get the KIND slot value.
pub fn get_kind(r: &Referent) -> MailKind {
    match r.get_string_value(ATTR_KIND) {
        Some("HEAD")      => MailKind::Head,
        Some("HELLO")     => MailKind::Hello,
        Some("BODY")      => MailKind::Body,
        Some("TAIL")      => MailKind::Tail,
        _                 => MailKind::Undefined,
    }
}

/// Set the TEXT slot (clear existing).
pub fn set_text(r: &mut Referent, text: &str) {
    r.add_slot(ATTR_TEXT, SlotValue::Str(text.to_string()), true);
}

/// Get the TEXT slot value.
pub fn get_text(r: &Referent) -> Option<&str> {
    r.get_string_value(ATTR_TEXT)
}
