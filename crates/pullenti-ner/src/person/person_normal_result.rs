/// PersonNormalResult — mirrors `PersonNormalResult.cs`.

#[derive(Debug, Clone, PartialEq, Default)]
pub enum PersonNormalResult {
    #[default]
    Undefined,
    OK,
    Manual,
    NotPerson,
}
