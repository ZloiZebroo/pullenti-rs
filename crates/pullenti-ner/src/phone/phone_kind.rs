/// Phone number type (тип телефонного номера)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhoneKind {
    #[default]
    Undefined = 0,
    /// Home phone (домашний)
    Home = 1,
    /// Mobile phone (мобильный)
    Mobile = 2,
    /// Work phone (рабочий)
    Work = 3,
    /// Fax (факс)
    Fax = 4,
}

impl std::fmt::Display for PhoneKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhoneKind::Undefined => write!(f, "undefined"),
            PhoneKind::Home => write!(f, "home"),
            PhoneKind::Mobile => write!(f, "mobile"),
            PhoneKind::Work => write!(f, "work"),
            PhoneKind::Fax => write!(f, "fax"),
        }
    }
}
