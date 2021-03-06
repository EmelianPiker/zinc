//!
//! The witness allocating gadget.
//!

use num::BigInt;

use franklin_crypto::bellman::pairing::ff::Field;
use franklin_crypto::bellman::ConstraintSystem;
use franklin_crypto::circuit::Assignment;

use zinc_build::ScalarType;

use crate::error::RuntimeError;
use crate::gadgets;
use crate::gadgets::scalar::Scalar;
use crate::IEngine;

pub fn allocate<E, CS>(
    mut cs: CS,
    value: Option<&BigInt>,
    scalar_type: ScalarType,
) -> Result<Scalar<E>, RuntimeError>
where
    E: IEngine,
    CS: ConstraintSystem<E>,
{
    let fr = if let Some(bigint) = value {
        Some(gadgets::scalar::fr_bigint::bigint_to_fr::<E>(bigint).ok_or(
            RuntimeError::ValueOverflow {
                value: bigint.clone(),
                scalar_type: scalar_type.clone(),
            },
        )?)
    } else {
        None
    };

    let variable = cs.alloc(|| "variable", || fr.grab())?;
    let scalar = Scalar::new_unchecked_variable(fr, variable, scalar_type.clone());

    match scalar_type {
        ScalarType::Field => {
            // Create some constraints to avoid unconstrained variable errors.
            let one = Scalar::new_constant_fr(E::Fr::one(), ScalarType::Field);
            gadgets::arithmetic::add::add(cs.namespace(|| "dummy constraint"), &scalar, &one)?;
            Ok(scalar)
        }
        scalar_type => {
            let condition = Scalar::new_constant_fr(E::Fr::one(), ScalarType::Boolean);
            Scalar::conditional_type_check(
                cs.namespace(|| "type check"),
                &condition,
                &scalar,
                scalar_type,
            )
        }
    }
}
