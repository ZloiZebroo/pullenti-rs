pub mod titlepage_referent;
pub mod titlepage_analyzer;

pub use titlepage_analyzer::TitlePageAnalyzer;
pub use titlepage_referent::{
    OBJ_TYPENAME as TITLEPAGE_OBJ_TYPENAME,
    ATTR_NAME     as TITLEPAGE_ATTR_NAME,
    ATTR_TYPE     as TITLEPAGE_ATTR_TYPE,
    ATTR_AUTHOR   as TITLEPAGE_ATTR_AUTHOR,
    ATTR_SUPERVISOR as TITLEPAGE_ATTR_SUPERVISOR,
    ATTR_EDITOR   as TITLEPAGE_ATTR_EDITOR,
    ATTR_CONSULTANT as TITLEPAGE_ATTR_CONSULTANT,
    ATTR_OPPONENT as TITLEPAGE_ATTR_OPPONENT,
    ATTR_TRANSLATOR as TITLEPAGE_ATTR_TRANSLATOR,
    ATTR_AFFIRMANT as TITLEPAGE_ATTR_AFFIRMANT,
    ATTR_ORG      as TITLEPAGE_ATTR_ORG,
    ATTR_DATE     as TITLEPAGE_ATTR_DATE,
    ATTR_CITY     as TITLEPAGE_ATTR_CITY,
    ATTR_SPECIALITY as TITLEPAGE_ATTR_SPECIALITY,
    new_titlepage_referent,
    get_name as get_titlepage_name,
    get_title_type,
    get_speciality as get_titlepage_speciality,
    add_name as add_titlepage_name,
    add_title_type,
    set_speciality as set_titlepage_speciality,
};
