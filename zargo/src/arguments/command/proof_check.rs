//!
//! The Zargo project manager `proof-check` subcommand.
//!

use std::convert::TryFrom;
use std::path::PathBuf;

use failure::Fail;
use structopt::StructOpt;

use crate::arguments::command::IExecutable;
use crate::directory::build::Directory as BuildDirectory;
use crate::directory::build::Error as BuildDirectoryError;
use crate::directory::data::Directory as DataDirectory;
use crate::directory::data::Error as DataDirectoryError;
use crate::directory::source::Directory as SourceDirectory;
use crate::executable::compiler::Compiler;
use crate::executable::compiler::Error as CompilerError;
use crate::executable::virtual_machine::Error as VirtualMachineError;
use crate::executable::virtual_machine::VirtualMachine;
use crate::manifest::Error as ManifestError;
use crate::manifest::Manifest;

///
/// The Zargo project manager `proof-check` subcommand.
///
#[derive(Debug, StructOpt)]
#[structopt(
    about = "Runs the full project building, running, trusted setup, proving & verifying sequence"
)]
pub struct Command {
    /// The logging level value, which helps the logger to set the logging level.
    #[structopt(
        short = "v",
        parse(from_occurrences),
        help = "Shows verbose logs, use multiple times for more verbosity"
    )]
    pub verbosity: usize,

    /// The path to the Zargo project manifest file.
    #[structopt(
        long = "manifest-path",
        parse(from_os_str),
        help = "Path to Zargo.toml",
        default_value = zinc_const::path::MANIFEST,
    )]
    pub manifest_path: PathBuf,

    /// The contract method to call. Only for contracts.
    #[structopt(long = "method", help = "The contract method to call")]
    pub method: Option<String>,

    /// Whether to run the release version.
    #[structopt(long = "release", help = "Run the release build")]
    pub is_release: bool,
}

///
/// The Zargo project manager `proof-check` subcommand error.
///
#[derive(Debug, Fail)]
pub enum Error {
    /// The manifest file error.
    #[fail(display = "manifest file {}", _0)]
    ManifestFile(ManifestError),
    /// The project binary build directory error.
    #[fail(display = "build directory {}", _0)]
    BuildDirectory(BuildDirectoryError),
    /// The project template, keys, and other auxiliary data directory error.
    #[fail(display = "data directory {}", _0)]
    DataDirectory(DataDirectoryError),
    /// The compiler process error.
    #[fail(display = "compiler {}", _0)]
    Compiler(CompilerError),
    /// The virtual machine `run` process error.
    #[fail(display = "virtual machine 'run' {}", _0)]
    VirtualMachineRun(VirtualMachineError),
    /// The virtual machine `setup` process error.
    #[fail(display = "virtual machine 'setup' {}", _0)]
    VirtualMachineSetup(VirtualMachineError),
    /// The virtual machine `proof-check` process error.
    #[fail(display = "virtual machine 'prove & verify' {}", _0)]
    VirtualMachineProveAndVerify(VirtualMachineError),
}

impl IExecutable for Command {
    type Error = Error;

    fn execute(self) -> Result<(), Self::Error> {
        let manifest = Manifest::try_from(&self.manifest_path).map_err(Error::ManifestFile)?;

        let mut manifest_path = self.manifest_path.clone();
        if manifest_path.is_file() {
            manifest_path.pop();
        }

        let source_directory_path = SourceDirectory::path(&manifest_path);

        DataDirectory::create(&manifest_path).map_err(Error::DataDirectory)?;
        let data_directory_path = DataDirectory::path(&manifest_path);
        let mut witness_path = data_directory_path.clone();
        let mut public_data_path = data_directory_path.clone();
        if let Some(ref method) = self.method {
            witness_path.push(format!(
                "{}_{}.{}",
                zinc_const::file_name::WITNESS,
                method,
                zinc_const::extension::JSON,
            ));
            public_data_path.push(format!(
                "{}_{}.{}",
                zinc_const::file_name::PUBLIC_DATA,
                method,
                zinc_const::extension::JSON,
            ));
        } else {
            witness_path.push(format!(
                "{}.{}",
                zinc_const::file_name::WITNESS,
                zinc_const::extension::JSON,
            ));
            public_data_path.push(format!(
                "{}.{}",
                zinc_const::file_name::PUBLIC_DATA,
                zinc_const::extension::JSON,
            ));
        }
        let mut storage_path = data_directory_path.clone();
        storage_path.push(format!(
            "{}.{}",
            zinc_const::file_name::STORAGE,
            zinc_const::extension::JSON
        ));
        let mut proving_key_path = data_directory_path.clone();
        proving_key_path.push(zinc_const::file_name::PROVING_KEY);
        let mut verifying_key_path = data_directory_path.clone();
        verifying_key_path.push(format!(
            "{}.{}",
            zinc_const::file_name::VERIFYING_KEY,
            zinc_const::extension::VERIFYING_KEY
        ));

        BuildDirectory::create(&manifest_path).map_err(Error::BuildDirectory)?;
        let build_directory_path = BuildDirectory::path(&manifest_path);
        let mut binary_path = build_directory_path;
        binary_path.push(format!(
            "{}.{}",
            zinc_const::file_name::BINARY,
            zinc_const::extension::BINARY
        ));

        if self.is_release {
            Compiler::build_release(
                self.verbosity,
                manifest.project.name.as_str(),
                &data_directory_path,
                &source_directory_path,
                &binary_path,
                false,
            )
            .map_err(Error::Compiler)?;
        } else {
            Compiler::build_debug(
                self.verbosity,
                manifest.project.name.as_str(),
                &data_directory_path,
                &source_directory_path,
                &binary_path,
                false,
            )
            .map_err(Error::Compiler)?;
        }

        match self.method {
            Some(method) => {
                VirtualMachine::run_contract(
                    self.verbosity,
                    &binary_path,
                    &witness_path,
                    &public_data_path,
                    &storage_path,
                    method.as_str(),
                )
                .map_err(Error::VirtualMachineRun)?;

                VirtualMachine::setup_contract(
                    self.verbosity,
                    &binary_path,
                    method.as_str(),
                    &proving_key_path,
                    &verifying_key_path,
                )
                .map_err(Error::VirtualMachineSetup)?;

                VirtualMachine::prove_and_verify_contract(
                    self.verbosity,
                    &binary_path,
                    &witness_path,
                    &public_data_path,
                    &storage_path,
                    method.as_str(),
                    &proving_key_path,
                    &verifying_key_path,
                )
                .map_err(Error::VirtualMachineProveAndVerify)?;
            }
            None => {
                VirtualMachine::run_circuit(
                    self.verbosity,
                    &binary_path,
                    &witness_path,
                    &public_data_path,
                )
                .map_err(Error::VirtualMachineRun)?;

                VirtualMachine::setup_circuit(
                    self.verbosity,
                    &binary_path,
                    &proving_key_path,
                    &verifying_key_path,
                )
                .map_err(Error::VirtualMachineSetup)?;

                VirtualMachine::prove_and_verify_circuit(
                    self.verbosity,
                    &binary_path,
                    &witness_path,
                    &public_data_path,
                    &proving_key_path,
                    &verifying_key_path,
                )
                .map_err(Error::VirtualMachineProveAndVerify)?;
            }
        }

        Ok(())
    }
}
