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

    pub fn list(ctx: Context<List>, price: u64) -> Result<()> {
        ctx.accounts.listing(price, ctx.bumps)
    }

    pub fn buy(ctx: Context<Buy>) -> Result<()> {
        ctx.accounts.send_sol()?;
        ctx.accounts.receive_nft()?;
        ctx.accounts.receive_rewards()
    }

    pub fn make_offer(ctx: Context<MakeOffer>, amount: u64) -> Result<()> {
        ctx.accounts.make_offer(amount, ctx.bumps)
    }

    pub fn accept_offer(ctx: Context<BuyerOffer>) -> Result<()> {
        ctx.accounts.accept_offer()
    }
}
