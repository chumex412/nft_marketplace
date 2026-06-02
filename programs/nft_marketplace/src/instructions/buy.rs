use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to_checked, Mint, MintToChecked, TokenAccount, TokenInterface},
};
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    instructions::TransferV1CpiBuilder,
    ID as MPL_CORE_ID,
};

use crate::{
    Errors, Listing, MarketPlace, LISTING, MARKETPLACE, MINT_DECIMALS, REWARDS_MINT, TREASURY,
};

#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(
        seeds = [MARKETPLACE, marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, MarketPlace>,

    #[account(mut)]
    pub asset: Account<'info, BaseAssetV1>,

    pub collection: Option<Account<'info, BaseCollectionV1>>,

    #[account(
        mut,
        close = maker,
        seeds = [LISTING, asset.key().as_ref()],
        bump = listing.bump,
        has_one = maker,
        has_one = asset,
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        mut,
        seeds = [TREASURY, marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    #[account(
        mut,
        mint::decimals = MINT_DECIMALS,
        mint::authority = marketplace,
        seeds = [REWARDS_MINT, marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = rewards_mint,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_reward_ata: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    /// CHECK: Verified via address constraint against the known MPL Core program ID
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
}

impl<'info> Buy<'info> {
    pub fn send_sol(&mut self) -> Result<()> {
        let price = self.listing.price;
        let fee = (price as u128)
            .checked_mul(self.marketplace.fee as u128)
            .unwrap()
            .checked_div(10_000)
            .ok_or(Errors::MathOverflow)? as u64;

        let maker_amount = price.checked_sub(fee).ok_or(Errors::MathOverflow)?;

        let initial_accounts = Transfer {
            from: self.taker.to_account_info(),
            to: self.maker.to_account_info(),
        };

        let initial_cpi_context =
            CpiContext::new(self.system_program.to_account_info(), initial_accounts);

        transfer(initial_cpi_context, maker_amount)?;

        let next_accounts = Transfer {
            from: self.taker.to_account_info(),
            to: self.treasury.to_account_info(),
        };

        let next_cpi_context =
            CpiContext::new(self.system_program.to_account_info(), next_accounts);

        transfer(next_cpi_context, fee)
    }

    pub fn receive_nft(&mut self) -> Result<()> {
        let asset_key = self.asset.key();
        let bump = self.listing.bump;
        let seeds = &[LISTING, asset_key.as_ref(), &[bump]];
        let signer_seeds = &[&seeds[..]];

        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(self.collection.as_ref().map(|c| c.as_ref()))
            .payer(&self.taker.to_account_info())
            .authority(Some(&self.listing.to_account_info()))
            .new_owner(&self.taker.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(signer_seeds)?;

        Ok(())
    }

    pub fn receive_rewards(&mut self) -> Result<()> {
        let price = self.listing.price;

        let signer_seeds: &[&[&[u8]]] = &[&[
            MARKETPLACE,
            self.marketplace.name.as_bytes(),
            &[self.marketplace.bump],
        ]];

        mint_to_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintToChecked {
                    mint: self.rewards_mint.to_account_info(),
                    to: self.taker_reward_ata.to_account_info(),
                    authority: self.marketplace.to_account_info(),
                },
                signer_seeds,
            ),
            price,
            MINT_DECIMALS,
        )?;

        Ok(())
    }
}
