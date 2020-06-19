//!
//! The Zinc VM bytecode program.
//!

pub mod circuit;
pub mod contract;
pub mod unit_test;

use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::data::r#type::Type as DataType;
use crate::instructions::Instruction;

use self::circuit::Circuit;
use self::contract::Contract;
use self::unit_test::UnitTest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Program {
    Circuit(Circuit),
    Contract(Contract),
    UnitTest(UnitTest),
}

impl Program {
    pub fn new_circuit(input: DataType, output: DataType, instructions: Vec<Instruction>) -> Self {
        Self::Circuit(Circuit::new(input, output, instructions))
    }

    pub fn new_contract(
        input: DataType,
        output: DataType,
        instructions: Vec<Instruction>,
        storage: Vec<(String, DataType)>,
    ) -> Self {
        Self::Contract(Contract::new(
            input,
            output.into_contract_metadata(),
            instructions,
            storage,
        ))
    }

    pub fn new_unit_test(instructions: Vec<Instruction>) -> Self {
        Self::UnitTest(UnitTest::new(instructions))
    }

    pub fn input(&self) -> DataType {
        match self {
            Self::Circuit(ref inner) => inner.input.to_owned(),
            Self::Contract(ref inner) => inner.input.to_owned(),
            Self::UnitTest(_) => DataType::new_empty_structure(),
        }
    }

    pub fn output(&self) -> DataType {
        match self {
            Self::Circuit(ref inner) => inner.output.to_owned(),
            Self::Contract(ref inner) => inner.output.to_owned(),
            Self::UnitTest(_) => DataType::Unit,
        }
    }

    pub fn instructions(&self) -> &[Instruction] {
        match self {
            Self::Circuit(ref inner) => inner.instructions.as_slice(),
            Self::Contract(ref inner) => inner.instructions.as_slice(),
            Self::UnitTest(ref inner) => inner.instructions.as_slice(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        bincode::deserialize(bytes).map_err(|e| format!("{:?}", e))
    }

    pub fn into_bytes(self) -> Vec<u8> {
        bincode::serialize(&self).expect(crate::panic::BINARY_SERIALIZATION)
    }
}
