use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    instructions::TransferV1CpiBuilder,
    ID as MPL_CORE_ID,
};

use crate::{Listing, LISTING};

#[derive(Accounts)]
pub struct List<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mut)]
    pub asset: Account<'info, BaseAssetV1>,

    #[account(mut)]
    pub collection: Option<Account<'info, BaseCollectionV1>>,

    #[account(
        init,
        space = Listing::DISCRIMINATOR.len() + Listing::INIT_SPACE,
        payer = maker,
        seeds = [LISTING, asset.key().as_ref()],
        bump
    )]
    pub listing: Account<'info, Listing>,

    pub payment_mint: Option<InterfaceAccount<'info, Mint>>,

    pub system_program: Program<'info, System>,

    /// CHECK: This is the ID of the MPL Core Program
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
}

impl<'info> List<'info> {
    pub fn listing(&mut self, price: u64, bumps: ListBumps) -> Result<()> {
        self.listing.set_inner(Listing {
            maker: self.maker.key(),
            asset: self.asset.key(),
            price,
            bump: bumps.listing,
            payment_mint: self.payment_mint.as_ref().map(|mint| mint.key()),
        });

        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(self.collection.as_ref().map(|c| c.as_ref()))
            .payer(&self.maker.to_account_info())
            .authority(Some(&self.maker.to_account_info()))
            .new_owner(&self.listing.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke()?;

        Ok(())
    }
}
