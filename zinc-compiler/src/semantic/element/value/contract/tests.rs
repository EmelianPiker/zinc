//!
//! The contract value element tests.
//!

#![cfg(test)]

use crate::error::Error;
use crate::lexical::token::location::Location;
use crate::semantic::element::error::Error as ElementError;
use crate::semantic::element::r#type::Type;
use crate::semantic::element::value::contract::error::Error as ContractValueError;
use crate::semantic::element::value::error::Error as ValueError;
use crate::semantic::error::Error as SemanticError;

#[test]
fn ok_not_initialized() {
    let input = r#"
contract Test {
    pub fn main() -> Self { Self }
}
"#;

    assert!(crate::semantic::tests::compile_entry(input).is_ok());
}

#[test]
fn error_not_initialized() {
    let input = r#"
contract Test {
    a: u8;
    b: u8;

    pub fn main() -> Self { Self }
}
"#;

    let expected = Err(Error::Semantic(SemanticError::Element(
        ElementError::Value(ValueError::Contract(ContractValueError::NotInitialized {
            location: Location::new(6, 29),
            type_identifier: "Test".to_owned(),
        })),
    )));

    let result = crate::semantic::tests::compile_entry(input);

    assert_eq!(result, expected);
}

#[test]
fn error_field_does_not_exist() {
    let input = r#"
contract Test {
    a: u8;
    b: u8;

    fn main() -> u8 { Self { a: 5, b: 10 }.c }
}
"#;

    let expected = Err(Error::Semantic(SemanticError::Element(
        ElementError::Value(ValueError::Contract(
            ContractValueError::FieldDoesNotExist {
                location: Location::new(6, 44),
                type_identifier: "Test".to_owned(),
                field_name: "c".to_owned(),
            },
        )),
    )));

    let result = crate::semantic::tests::compile_entry(input);

    assert_eq!(result, expected);
}

#[test]
fn error_field_expected() {
    let input = r#"
contract Test {
    a: u8;
    b: u8;

    fn main() -> Self { Self { a: 5, c: 10 } }
}
"#;

    let expected = Err(Error::Semantic(SemanticError::Element(
        ElementError::Value(ValueError::Contract(ContractValueError::FieldExpected {
            location: Location::new(6, 38),
            type_identifier: "Test".to_owned(),
            position: 2,
            expected: "b".to_owned(),
            found: "c".to_owned(),
        })),
    )));

    let result = crate::semantic::tests::compile_entry(input);

    assert_eq!(result, expected);
}

#[test]
fn error_field_invalid_type() {
    let input = r#"
contract Test {
    a: u8;
    b: u8;

    fn main() -> Self { Self { a: 5, b: true } }
}
"#;

    let expected = Err(Error::Semantic(SemanticError::Element(
        ElementError::Value(ValueError::Contract(ContractValueError::FieldInvalidType {
            location: Location::new(6, 38),
            type_identifier: "Test".to_owned(),
            field_name: "b".to_owned(),
            expected: Type::integer_unsigned(None, zinc_const::BITLENGTH_BYTE).to_string(),
            found: Type::boolean(None).to_string(),
        })),
    )));

    let result = crate::semantic::tests::compile_entry(input);

    assert_eq!(result, expected);
}

#[test]
fn error_field_out_of_range() {
    let input = r#"
contract Test {
    a: u8;
    b: u8;

    fn main() -> Self { Self { a: 5, b: 10, c: 15 } }
}
"#;

    let expected = Err(Error::Semantic(SemanticError::Element(
        ElementError::Value(ValueError::Contract(ContractValueError::FieldOutOfRange {
            location: Location::new(6, 45),
            type_identifier: "Test".to_owned(),
            expected: 2,
            found: 3,
        })),
    )));

    let result = crate::semantic::tests::compile_entry(input);

    assert_eq!(result, expected);
}