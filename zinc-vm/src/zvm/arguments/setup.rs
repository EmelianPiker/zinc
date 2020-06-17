//!
//! The Zinc virtual machine binary `setup` command.
//!

use std::fs;
use std::path::PathBuf;

use structopt::StructOpt;

use franklin_crypto::bellman::pairing::bn256::Bn256;

use zinc_bytecode::Program as BytecodeProgram;

use zinc_vm::IFacade;

use crate::error::Error;
use crate::error::IoToError;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "setup",
    about = "Generates a pair of proving and verifying keys"
)]
pub struct SetupCommand {
    #[structopt(long = "binary", help = "The bytecode file")]
    pub binary_path: PathBuf,

    #[structopt(long = "proving-key", help = "The proving key path")]
    pub proving_key_path: PathBuf,

    #[structopt(long = "verifying-key", help = "The verifying key path")]
    pub verifying_key_path: PathBuf,
}

impl SetupCommand {
    pub fn execute(&self) -> Result<(), Error> {
        let bytes =
            fs::read(&self.binary_path).error_with_path(|| self.binary_path.to_string_lossy())?;
        let program =
            BytecodeProgram::from_bytes(bytes.as_slice()).map_err(Error::ProgramDecoding)?;

        let params = program.setup::<Bn256>()?;

        let pkey_file = fs::File::create(&self.proving_key_path)
            .error_with_path(|| self.proving_key_path.to_string_lossy())?;
        params
            .write(pkey_file)
            .error_with_path(|| self.proving_key_path.to_string_lossy())?;

        let vk_hex = {
            let mut vk_bytes = Vec::new();
            params.vk.write(&mut vk_bytes).expect("writing to vec");
            hex::encode(vk_bytes) + "\n"
        };

        fs::write(&self.verifying_key_path, vk_hex)
            .error_with_path(|| self.verifying_key_path.to_string_lossy())?;

        Ok(())
    }
}
