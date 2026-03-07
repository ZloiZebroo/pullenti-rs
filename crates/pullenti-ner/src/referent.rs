use std::any::Any;
use std::rc::Rc;
use std::cell::RefCell;

/// A text fragment annotation (span where an entity occurs in text)
#[derive(Debug, Clone)]
pub struct TextAnnotation {
    /// Start character position (inclusive)
    pub begin_char: i32,
    /// End character position (inclusive)
    pub end_char: i32,
    /// Whether this annotation is essential for the entity occurrence
    pub essential_for_occurrence: bool,
}

impl TextAnnotation {
    pub fn new(begin: i32, end: i32) -> Self {
        TextAnnotation { begin_char: begin, end_char: end, essential_for_occurrence: false }
    }

    pub fn get_text<'a>(&self, text: &'a str) -> Option<&'a str> {
        let b = self.begin_char as usize;
        let e = (self.end_char + 1) as usize;
        if b < e && e <= text.len() {
            Some(&text[b..e])
        } else {
            None
        }
    }
}

/// The value stored in a Slot — either a nested Referent, a string, or other data
#[derive(Debug, Clone)]
pub enum SlotValue {
    Str(String),
    Referent(Rc<RefCell<Referent>>),
}

impl SlotValue {
    pub fn as_str(&self) -> Option<&str> {
        match self { SlotValue::Str(s) => Some(s.as_str()), _ => None }
    }

    pub fn as_referent(&self) -> Option<Rc<RefCell<Referent>>> {
        match self {
            SlotValue::Referent(r) => Some(r.clone()),
            _ => None,
        }
    }
}

impl std::fmt::Display for SlotValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlotValue::Str(s) => write!(f, "{}", s),
            SlotValue::Referent(r) => write!(f, "{}", r.borrow().to_string()),
        }
    }
}

/// A named attribute slot on a Referent entity
#[derive(Debug, Clone)]
pub struct Slot {
    /// Attribute name
    pub type_name: String,
    /// Attribute value
    pub value: Option<SlotValue>,
    /// Occurrence count (statistical use)
    pub count: i32,
    /// Text positions where this slot value was found
    pub occurrence: Vec<TextAnnotation>,
}

impl Slot {
    pub fn new(type_name: impl Into<String>, value: Option<SlotValue>) -> Self {
        Slot {
            type_name: type_name.into(),
            value,
            count: 1,
            occurrence: Vec::new(),
        }
    }

    pub fn is_internal(&self) -> bool {
        self.type_name.starts_with('@')
    }

    pub fn add_annotation(&mut self, begin: i32, end: i32) {
        if !self.occurrence.iter().any(|o| o.begin_char == begin && o.end_char == end) {
            self.occurrence.push(TextAnnotation::new(begin, end));
        }
    }
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(v) => write!(f, "{}: {}", self.type_name, v),
            None => write!(f, "{}: (none)", self.type_name),
        }
    }
}

/// A named entity (ИМЯ СУЩНОСТИ) — base type for all recognized entities
pub struct Referent {
    /// Entity type name (e.g. "PERSON", "ORGANIZATION", "GEO", etc.)
    pub type_name: String,
    /// Attribute slots
    pub slots: Vec<Slot>,
    /// Text positions where this entity occurs
    pub occurrence: Vec<TextAnnotation>,
    /// Subtype-specific data (downcastable via Any)
    pub data: Box<dyn Any>,
}

impl std::fmt::Debug for Referent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Referent({})", self.type_name)
    }
}

impl Referent {
    pub fn new(type_name: impl Into<String>) -> Self {
        Referent {
            type_name: type_name.into(),
            slots: Vec::new(),
            occurrence: Vec::new(),
            data: Box::new(()),
        }
    }

    pub fn new_with_data<T: Any>(type_name: impl Into<String>, data: T) -> Self {
        Referent {
            type_name: type_name.into(),
            slots: Vec::new(),
            occurrence: Vec::new(),
            data: Box::new(data),
        }
    }

    /// Add or update a slot value. If `clear_old` is true, remove any existing slot with the same name.
    pub fn add_slot(&mut self, name: impl Into<String>, value: SlotValue, clear_old: bool) -> &mut Slot {
        let name = name.into();
        if clear_old {
            self.slots.retain(|s| s.type_name != name);
        }
        // Check if same value already exists
        for slot in &mut self.slots {
            if slot.type_name == name {
                if slot.value.as_ref().map_or(false, |v| v.to_string() == value.to_string()) {
                    slot.count += 1;
                    let len = self.slots.len();
                    return &mut self.slots[len - 1]; // return last (hack, will fix)
                }
            }
        }
        self.slots.push(Slot::new(name, Some(value)));
        let idx = self.slots.len() - 1;
        &mut self.slots[idx]
    }

    /// Find a slot by name (and optionally by value)
    pub fn find_slot(&self, name: &str, value: Option<&str>) -> Option<&Slot> {
        self.slots.iter().find(|s| {
            s.type_name == name
                && match value {
                    None => true,
                    Some(v) => s.value.as_ref().map_or(false, |sv| sv.to_string() == v),
                }
        })
    }

    /// Get the first string value of a named slot
    pub fn get_string_value(&self, name: &str) -> Option<&str> {
        self.slots.iter()
            .find(|s| s.type_name == name)
            .and_then(|s| s.value.as_ref())
            .and_then(|v| v.as_str())
    }

    /// Get all string values for a named slot
    pub fn get_all_string_values(&self, name: &str) -> Vec<&str> {
        self.slots.iter()
            .filter(|s| s.type_name == name)
            .filter_map(|s| s.value.as_ref().and_then(|v| v.as_str()))
            .collect()
    }

    /// Add occurrence annotation
    pub fn add_occurrence(&mut self, begin: i32, end: i32) {
        if !self.occurrence.iter().any(|o| o.begin_char == begin && o.end_char == end) {
            self.occurrence.push(TextAnnotation::new(begin, end));
        }
    }

    /// Get subtype data as concrete type
    pub fn data_as<T: Any>(&self) -> Option<&T> {
        self.data.downcast_ref::<T>()
    }

    pub fn data_as_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.data.downcast_mut::<T>()
    }
}

impl std::fmt::Display for Referent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slots_str: Vec<String> = self.slots.iter()
            .filter(|s| !s.is_internal())
            .map(|s| s.to_string())
            .collect();
        write!(f, "{}[{}]", self.type_name, slots_str.join("; "))
    }
}
