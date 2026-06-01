use anchor_lang::prelude::*;

#[error_code]

pub enum Errors {
    #[msg("Name is empty or too long")]
    InvalidName,
    #[msg("A mathetical overflow was detected")]
    MathOverflow,
}
