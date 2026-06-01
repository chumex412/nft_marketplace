use anchor_lang::prelude::*;

use crate::MAX_NAME_LENGTH;

#[account]
#[derive(InitSpace)]
pub struct MarketPlace {
    pub admin: Pubkey,
    pub fee: u16,
    #[max_len(MAX_NAME_LENGTH)]
    pub name: String,
    pub bump: u8,
    pub treasury_bump: u8,
    pub rewards_bump: u8,
}
