//! Runtime support for generated request-field normalization.

use convert_case::{Case, Casing};
use serde::Deserialize;

use super::maybe::Maybe;

#[derive(Clone, Copy)]
pub enum Operation {
    Trim,
    TrimStart,
    TrimEnd,
    CollapseWhitespace,
    Lowercase,
    Uppercase,
    CamelCase,
    PascalCase,
    SnakeCase,
    KebabCase,
    ScreamingSnakeCase,
    ScreamingKebabCase,
    TitleCase,
    SentenceCase,
}

pub trait Normalize {
    fn normalize(self, operations: &[Operation]) -> Self;
}

impl Normalize for String {
    fn normalize(mut self, operations: &[Operation]) -> Self {
        for operation in operations {
            self = match operation {
                Operation::Trim => self.trim().to_owned(),
                Operation::TrimStart => self.trim_start().to_owned(),
                Operation::TrimEnd => self.trim_end().to_owned(),
                Operation::CollapseWhitespace => {
                    self.split_whitespace().collect::<Vec<_>>().join(" ")
                }
                Operation::Lowercase => self.to_lowercase(),
                Operation::Uppercase => self.to_uppercase(),
                Operation::CamelCase => self.to_case(Case::Camel),
                Operation::PascalCase => self.to_case(Case::Pascal),
                Operation::SnakeCase => self.to_case(Case::Snake),
                Operation::KebabCase => self.to_case(Case::Kebab),
                Operation::ScreamingSnakeCase => self.to_case(Case::UpperSnake),
                Operation::ScreamingKebabCase => self.to_case(Case::UpperKebab),
                Operation::TitleCase => self.to_case(Case::Title),
                Operation::SentenceCase => self.to_case(Case::Sentence),
            };
        }
        self
    }
}

impl<T: Normalize> Normalize for Option<T> {
    fn normalize(self, operations: &[Operation]) -> Self {
        self.map(|value| value.normalize(operations))
    }
}

impl<T: Normalize> Normalize for Maybe<T> {
    fn normalize(self, operations: &[Operation]) -> Self {
        self.map(|value| value.normalize(operations))
    }
}

pub fn deserialize<'de, D, T>(deserializer: D, operations: &[Operation]) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Normalize,
{
    T::deserialize(deserializer).map(|value| value.normalize(operations))
}
