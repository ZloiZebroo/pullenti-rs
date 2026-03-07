/// Уточнение позиции в периоде (начало, середина, конец, ...)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DatePointerType {
    #[default]
    No,
    Begin,
    Center,
    End,
    Today,
    Winter,
    Spring,
    Summer,
    Autumn,
    About,
    Undefined,
}

impl std::fmt::Display for DatePointerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DatePointerType::No        => "No",
            DatePointerType::Begin     => "Begin",
            DatePointerType::Center    => "Center",
            DatePointerType::End       => "End",
            DatePointerType::Today     => "Today",
            DatePointerType::Winter    => "Winter",
            DatePointerType::Spring    => "Spring",
            DatePointerType::Summer    => "Summer",
            DatePointerType::Autumn    => "Autumn",
            DatePointerType::About     => "About",
            DatePointerType::Undefined => "Undefined",
        };
        write!(f, "{}", s)
    }
}
