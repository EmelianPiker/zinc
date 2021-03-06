//!
//! The Zinc virtual machine subcommand.
//!

pub mod prove;
pub mod run;
pub mod setup;
pub mod test;
pub mod verify;

use structopt::StructOpt;

use crate::error::Error;

use self::prove::Command as ProveCommand;
use self::run::Command as RunCommand;
use self::setup::Command as SetupCommand;
use self::test::Command as TestCommand;
use self::verify::Command as VerifyCommand;

///
/// The generic trait used for commands.
///
pub trait IExecutable {
    /// The generic subcommand error type.
    type Error;

    ///
    /// Executes the instance.
    ///
    fn execute(self) -> Result<i32, Self::Error>;
}

///
/// The Zinc virtual machine subcommand.
///
#[derive(Debug, StructOpt)]
#[structopt(about = "The Zinc virtual machine")]
pub enum Command {
    /// Executes the bytecode and prints its output.
    Run(RunCommand),
    /// Executes a unit test.
    Test(TestCommand),
    /// Generates a pair of proving and verifying keys.
    Setup(SetupCommand),
    /// Generates a proof using the proving key.
    Prove(ProveCommand),
    /// Verifies a proof using the verifying key.
    Verify(VerifyCommand),
}

impl IExecutable for Command {
    type Error = Error;

    fn execute(self) -> Result<i32, Self::Error> {
        match self {
            Command::Run(inner) => inner.execute(),
            Command::Test(inner) => inner.execute(),
            Command::Setup(inner) => inner.execute(),
            Command::Prove(inner) => inner.execute(),
            Command::Verify(inner) => inner.execute(),
        }
    }
}
