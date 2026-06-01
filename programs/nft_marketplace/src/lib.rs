use anchor_lang::prelude::*;

declare_id!("FnLm3K6usf6Wd2AM5gsdtfepeakTqYi4FcQGYGBZLGmG");

pub mod instructions;
pub mod state;
pub mod constant;
pub mod errors;

pub use instructions::*;
pub use state::*;
pub use constant::*;
pub use errors::*;

#[program]
pub mod nft_marketplace {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, name: String, fee: u16) -> Result<()> {
        ctx.accounts.init(name, fee, ctx.bumps)
    }
}
