use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};

use crate::{
    Errors, MarketPlace, MARKETPLACE, MAX_NAME_LENGTH, MINT_DECIMALS, REWARDS_MINT, TREASURY,
};

#[derive(Accounts)]
#[instruction(name: String)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = MarketPlace::DISCRIMINATOR.len() + MarketPlace::INIT_SPACE,
        seeds = [MARKETPLACE, name.as_str().as_bytes()],
        bump,
    )]
    pub marketplace: Account<'info, MarketPlace>,

    #[account(
        seeds = [TREASURY, marketplace.key().as_ref()],
        bump,
    )]
    pub treasury: SystemAccount<'info>,

    #[account(
        init,
        payer = admin,
        mint::decimals = MINT_DECIMALS,
        mint::authority = marketplace,
        seeds = [REWARDS_MINT, marketplace.key().as_ref()],
        bump
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Initialize<'info> {
    pub fn init(&mut self, name: String, fee: u16, bumps: InitializeBumps) -> Result<()> {
        require!(
            !name.trim().is_empty() && name.len() as u8 <= MAX_NAME_LENGTH,
            Errors::InvalidName
        );

        self.marketplace.set_inner(MarketPlace {
            admin: self.admin.key(),
            fee,
            name,
            bump: bumps.marketplace,
            treasury_bump: bumps.treasury,
            rewards_bump: bumps.rewards_mint,
        });

        Ok(())
    }
}
