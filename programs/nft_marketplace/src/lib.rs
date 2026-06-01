use anchor_lang::prelude::*;

declare_id!("FnLm3K6usf6Wd2AM5gsdtfepeakTqYi4FcQGYGBZLGmG");

#[program]
pub mod nft_marketplace {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
