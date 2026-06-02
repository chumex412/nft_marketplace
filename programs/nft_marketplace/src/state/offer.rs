use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Offer {
    pub taker: Pubkey,
    pub asset: Pubkey,
    pub payment_mint: Pubkey,
    pub amount: u64,
    pub bump: u8,
}
