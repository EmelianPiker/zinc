//!
//! The Zinc VM bytecode.
//!

pub mod entry;
pub mod metadata;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use zinc_bytecode::Instruction;
use zinc_bytecode::Program as BytecodeProgram;
use zinc_bytecode::TemplateValue;

use crate::generator::r#type::Type;
use crate::lexical::token::location::Location;
use crate::source::file::index::INDEX as FILE_INDEX;

use self::entry::Entry;
use self::metadata::Metadata;

///
/// The Zinc VM bytecode, generated by the compiler target code generator.
///
#[derive(Debug, PartialEq)]
pub struct Bytecode {
    /// The Zinc VM instructions written by the bytecode generator
    instructions: Vec<Instruction>,

    /// The contract storage structure
    contract_storage: Option<Vec<(String, Type)>>,
    /// Metadata of each application entry
    entry_metadata: HashMap<usize, Metadata>,
    /// Unit test function IDs
    test_entries: HashMap<usize, String>,

    /// Data stack addresses of variables declared at runtime
    variable_addresses: HashMap<String, usize>,
    /// Bytecode addresses of the functions written to the bytecode
    function_addresses: HashMap<usize, usize>,
    /// The pointer which is reset at the beginning of each function
    data_stack_pointer: usize,

    /// The location pointer used to pass debug information to the VM
    current_location: Location,
}

impl Default for Bytecode {
    fn default() -> Self {
        Self::new()
    }
}

impl Bytecode {
    const INSTRUCTIONS_INITIAL_CAPACITY: usize = 1024;
    const FUNCTION_ADDRESSES_INITIAL_CAPACITY: usize = 16;
    const VARIABLE_ADDRESSES_INITIAL_CAPACITY: usize = 16;
    const ENTRY_METADATA_INITIAL_CAPACITY: usize = 16;

    ///
    /// Creates a new bytecode instance with the placeholders for the entry `Call` and
    /// `Exit` instructions.
    ///
    pub fn new() -> Self {
        let mut instructions = Vec::with_capacity(Self::INSTRUCTIONS_INITIAL_CAPACITY);
        instructions.push(Instruction::NoOperation(zinc_bytecode::NoOperation));
        instructions.push(Instruction::NoOperation(zinc_bytecode::NoOperation));

        Self {
            instructions,

            contract_storage: None,
            entry_metadata: HashMap::with_capacity(Self::ENTRY_METADATA_INITIAL_CAPACITY),
            test_entries: HashMap::with_capacity(Self::ENTRY_METADATA_INITIAL_CAPACITY),

            variable_addresses: HashMap::with_capacity(Self::VARIABLE_ADDRESSES_INITIAL_CAPACITY),
            function_addresses: HashMap::with_capacity(Self::FUNCTION_ADDRESSES_INITIAL_CAPACITY),
            data_stack_pointer: 0,

            current_location: Location::new_beginning(None),
        }
    }

    ///
    /// Wraps the bytecode into `Rc<RefCell<_>>` simplifying most of initializations.
    ///
    pub fn wrap(self) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(self))
    }

    ///
    /// Extracts the bytecode from `Rc<RefCell<_>>`.
    ///
    pub fn unwrap_rc(bytecode: Rc<RefCell<Self>>) -> Self {
        Rc::try_unwrap(bytecode)
            .expect(crate::panic::LAST_SHARED_REFERENCE)
            .into_inner()
    }

    ///
    /// Returns the variable address in the function data stack frame.
    ///
    pub fn get_variable_address(&self, name: &str) -> Option<usize> {
        self.variable_addresses.get(name).copied()
    }

    pub fn set_contract_storage(&mut self, fields: Vec<(String, Type)>) {
        self.contract_storage = Some(fields);
    }

    ///
    /// Starts a new function, resetting the data stack pointer and writing the
    /// function debug information.
    ///
    pub fn start_function(&mut self, location: Location, type_id: usize, identifier: String) {
        let address = self.instructions.len();
        self.function_addresses.insert(type_id, address);
        self.data_stack_pointer = 0;

        let file_path = FILE_INDEX
            .get_path(location.file_index)
            .to_string_lossy()
            .to_string();
        self.instructions
            .push(Instruction::FileMarker(zinc_bytecode::FileMarker::new(
                file_path,
            )));
        self.instructions.push(Instruction::FunctionMarker(
            zinc_bytecode::FunctionMarker::new(identifier),
        ));
    }

    ///
    /// Starts an entry function, saves its metadata and calls the `start_function` method.
    ///
    pub fn start_entry_function(
        &mut self,
        location: Location,
        type_id: usize,
        identifier: String,
        input_arguments: Vec<(String, Type)>,
        output_type: Option<Type>,
    ) {
        let metadata = Metadata::new(
            identifier.clone(),
            input_arguments,
            output_type.unwrap_or_else(|| Type::structure(vec![])),
        );
        self.entry_metadata.insert(type_id, metadata);

        self.start_function(location, type_id, identifier);
    }

    ///
    /// Starts a unit test function, saves its metadata and calls the `start_function` method.
    ///
    pub fn start_unit_test_function(
        &mut self,
        location: Location,
        type_id: usize,
        identifier: String,
    ) {
        self.test_entries.insert(type_id, identifier.clone());

        self.start_function(location, type_id, identifier);
    }

    ///
    /// Defines a variable, saving its address within the current data stack frame.
    ///
    pub fn define_variable(&mut self, identifier: Option<String>, size: usize) -> usize {
        let start_address = self.data_stack_pointer;
        if let Some(identifier) = identifier {
            self.variable_addresses
                .insert(identifier, self.data_stack_pointer);
        }
        self.data_stack_pointer += size;
        start_address
    }

    ///
    /// Writes the instruction along with its location debug information.
    ///
    pub fn push_instruction(&mut self, instruction: Instruction, location: Option<Location>) {
        if let Some(location) = location {
            if self.current_location != location {
                if self.current_location.line != location.line {
                    self.instructions.push(Instruction::LineMarker(
                        zinc_bytecode::LineMarker::new(location.line),
                    ));
                }
                if self.current_location.column != location.column {
                    self.instructions.push(Instruction::ColumnMarker(
                        zinc_bytecode::ColumnMarker::new(location.column),
                    ));
                }
                self.current_location = location;
            }
        }

        self.instructions.push(instruction)
    }

    ///
    /// Generates the bytecode for the entry specified with `entry_id`.
    ///
    /// Besides that, the function walks through the `Call` instructions and replaces `type_id`s
    /// of the function stored as `address`es with their actual addresses in the bytecode,
    /// since the addresses are only known after the functions are written thereto.
    ///
    pub fn into_entries(self) -> HashMap<String, Entry> {
        let mut data = HashMap::with_capacity(self.entry_metadata.len());

        for (entry_id, metadata) in self.entry_metadata.into_iter() {
            let mut instructions = self.instructions.clone();
            instructions[0] =
                Instruction::Call(zinc_bytecode::Call::new(entry_id, metadata.input_size()));
            instructions[1] = Instruction::Exit(zinc_bytecode::Exit::new(metadata.output_size()));

            for instruction in instructions.iter_mut() {
                if let Instruction::Call(zinc_bytecode::Call {
                    address: ref mut type_id,
                    ..
                }) = instruction
                {
                    *type_id = self
                        .function_addresses
                        .get(&type_id)
                        .copied()
                        .expect(crate::panic::VALIDATED_DURING_SEMANTIC_ANALYSIS);
                }
            }

            for (index, instruction) in instructions.iter().enumerate() {
                match instruction {
                    instruction @ Instruction::FileMarker(_)
                    | instruction @ Instruction::FunctionMarker(_)
                    | instruction @ Instruction::LineMarker(_)
                    | instruction @ Instruction::ColumnMarker(_) => {
                        log::trace!("{:03} {:?}", index, instruction)
                    }
                    instruction => log::debug!("{:03} {:?}", index, instruction),
                }
            }

            let bytecode = match self.contract_storage.as_ref() {
                Some(storage) => {
                    let storage = storage
                        .iter()
                        .map(|(name, r#type)| (name.to_owned(), r#type.to_owned().into()))
                        .collect();

                    BytecodeProgram::new_contract(
                        metadata.input_fields_as_struct().into(),
                        metadata.output_type.clone().into(),
                        instructions,
                        storage,
                    )
                }
                None => BytecodeProgram::new_circuit(
                    metadata.input_fields_as_struct().into(),
                    metadata.output_type.clone().into(),
                    instructions,
                ),
            }
            .into_bytes();

            let input_template_value = TemplateValue::new(metadata.input_fields_as_struct().into());
            let witness_template =
                match serde_json::to_string_pretty(&input_template_value.into_json()) {
                    Ok(json) => (json + "\n").into_bytes(),
                    Err(error) => panic!(
                        crate::panic::JSON_TEMPLATE_SERIALIZATION.to_owned()
                            + error.to_string().as_str()
                    ),
                };

            let output_value_template = TemplateValue::new(metadata.output_type.into());
            let public_data_template =
                match serde_json::to_string_pretty(&output_value_template.into_json()) {
                    Ok(json) => (json + "\n").into_bytes(),
                    Err(error) => panic!(
                        crate::panic::JSON_TEMPLATE_SERIALIZATION.to_owned()
                            + error.to_string().as_str()
                    ),
                };

            data.insert(
                metadata.entry_name,
                Entry::new(bytecode, witness_template, public_data_template),
            );
        }

        for (entry_id, name) in self.test_entries.into_iter() {
            let mut instructions = self.instructions.clone();
            instructions[0] = Instruction::Call(zinc_bytecode::Call::new(entry_id, 0));
            instructions[1] = Instruction::Exit(zinc_bytecode::Exit::new(0));

            for instruction in instructions.iter_mut() {
                if let Instruction::Call(zinc_bytecode::Call {
                    address: ref mut type_id,
                    ..
                }) = instruction
                {
                    *type_id = self
                        .function_addresses
                        .get(&type_id)
                        .copied()
                        .expect(crate::panic::VALIDATED_DURING_SEMANTIC_ANALYSIS);
                }
            }

            for (index, instruction) in instructions.iter().enumerate() {
                match instruction {
                    instruction @ Instruction::FileMarker(_)
                    | instruction @ Instruction::FunctionMarker(_)
                    | instruction @ Instruction::LineMarker(_)
                    | instruction @ Instruction::ColumnMarker(_) => {
                        log::trace!("{:03} {:?}", index, instruction)
                    }
                    instruction => log::debug!("{:03} {:?}", index, instruction),
                }
            }

            let bytecode = BytecodeProgram::new_unit_test(instructions).into_bytes();

            data.insert(name, Entry::new_test(bytecode));
        }

        data
    }
}
