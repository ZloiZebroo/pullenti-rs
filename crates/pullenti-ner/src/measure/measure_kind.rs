/// What category of physical quantity this measures — mirrors MeasureKind.cs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasureKind {
    Undefined,
    Time,
    Length,
    Area,
    Volume,
    Weight,
    Speed,
    Temperature,
    Ip,
    Percent,
    Money,
    Count,
    // extras from UnitsHelper
    Power,
    Energy,
    Voltage,
    Current,
    Resistance,
    Frequency,
    Pressure,
    Data,
    Radiation,
    Angle,
    Luminous,
    Force,
    Capacity,
    Other,
}

impl MeasureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Undefined    => "Undefined",
            Self::Time         => "Time",
            Self::Length       => "Length",
            Self::Area         => "Area",
            Self::Volume       => "Volume",
            Self::Weight       => "Weight",
            Self::Speed        => "Speed",
            Self::Temperature  => "Temperature",
            Self::Ip           => "Ip",
            Self::Percent      => "Percent",
            Self::Money        => "Money",
            Self::Count        => "Count",
            Self::Power        => "Power",
            Self::Energy       => "Energy",
            Self::Voltage      => "Voltage",
            Self::Current      => "Current",
            Self::Resistance   => "Resistance",
            Self::Frequency    => "Frequency",
            Self::Pressure     => "Pressure",
            Self::Data         => "Data",
            Self::Radiation    => "Radiation",
            Self::Angle        => "Angle",
            Self::Luminous     => "Luminous",
            Self::Force        => "Force",
            Self::Capacity     => "Capacity",
            Self::Other        => "Other",
        }
    }
}
