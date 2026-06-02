use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use mpl_core::accounts::BaseAssetV1;

use crate::{state::Offer, OFFER};

#[derive(Accounts)]
pub struct MakeOffer<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    pub asset: Account<'info, BaseAssetV1>,

    pub payment_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = taker,
        space = Offer::DISCRIMINATOR.len() + Offer::INIT_SPACE,
        seeds = [OFFER, asset.key().as_ref(), taker.key().as_ref()],
        bump,
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = payment_mint,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_payment_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = taker,
        associated_token::mint = payment_mint,
        associated_token::authority = offer,
        associated_token::token_program = token_program,
    )]
    pub offer_vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> MakeOffer<'info> {
    pub fn make_offer(&mut self, amount: u64, bumps: MakeOfferBumps) -> Result<()> {
        self.offer.set_inner(Offer {
            taker: self.taker.key(),
            asset: self.asset.key(),
            payment_mint: self.payment_mint.key(),
            amount,
            bump: bumps.offer,
        });

        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.taker_payment_ata.to_account_info(),
                    mint: self.payment_mint.to_account_info(),
                    to: self.offer_vault_ata.to_account_info(),
                    authority: self.taker.to_account_info(),
                },
            ),
            amount,
            self.payment_mint.decimals,
        )
    }
}
