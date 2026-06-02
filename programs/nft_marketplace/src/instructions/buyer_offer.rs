use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, mint_to_checked, transfer_checked, CloseAccount, Mint, MintToChecked,
        TokenAccount, TokenInterface, TransferChecked,
    },
};
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    instructions::TransferV1CpiBuilder,
    ID as MPL_CORE_ID,
};

use crate::{
    state::Offer, Errors, Listing, MarketPlace, LISTING, MARKETPLACE, MINT_DECIMALS, OFFER,
    REWARDS_MINT, TREASURY,
};

#[derive(Accounts)]
pub struct BuyerOffer<'info> {
    /// Seller — accepts the offer and receives tokens
    #[account(mut)]
    pub maker: Signer<'info>,

    /// Buyer — made the original offer, receives the NFT
    #[account(mut)]
    pub taker: SystemAccount<'info>,

    #[account(
        seeds = [MARKETPLACE, marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Box<Account<'info, MarketPlace>>,

    #[account(mut)]
    pub asset: Box<Account<'info, BaseAssetV1>>,

    pub collection: Option<Box<Account<'info, BaseCollectionV1>>>,

    #[account(
        mut,
        close = maker,
        seeds = [LISTING, asset.key().as_ref()],
        bump = listing.bump,
        has_one = maker,
        has_one = asset,
    )]
    pub listing: Box<Account<'info, Listing>>,

    #[account(
        mut,
        close = taker,
        seeds = [OFFER, asset.key().as_ref(), taker.key().as_ref()],
        bump = offer.bump,
        has_one = taker,
        constraint = offer.asset == asset.key(),
        constraint = offer.payment_mint == payment_mint.key(),
    )]
    pub offer: Box<Account<'info, Offer>>,

    pub payment_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = offer,
        associated_token::token_program = token_program,
    )]
    pub offer_vault_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = payment_mint,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_payment_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [TREASURY, marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = payment_mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_program,
    )]
    pub treasury_payment_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        mint::decimals = MINT_DECIMALS,
        mint::authority = marketplace,
        seeds = [REWARDS_MINT, marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
    )]
    pub rewards_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = maker,
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

impl<'info> BuyerOffer<'info> {
    pub fn accept_offer(&mut self) -> Result<()> {
        let amount = self.offer.amount;
        let fee = (amount as u128)
            .checked_mul(self.marketplace.fee as u128)
            .unwrap()
            .checked_div(10_000)
            .ok_or(Errors::MathOverflow)? as u64;
        let maker_amount = amount.checked_sub(fee).ok_or(Errors::MathOverflow)?;

        let asset_key = self.asset.key();
        let taker_key = self.taker.key();
        let offer_bump = self.offer.bump;
        let offer_seeds = &[OFFER, asset_key.as_ref(), taker_key.as_ref(), &[offer_bump]];
        let offer_signer = &[&offer_seeds[..]];

        let payment_accounts = TransferChecked {
            from: self.offer_vault_ata.to_account_info(),
            mint: self.payment_mint.to_account_info(),
            to: self.maker_payment_ata.to_account_info(),
            authority: self.offer.to_account_info(),
        };
        let payment_cpi_context = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            payment_accounts,
            offer_signer,
        );
        transfer_checked(
            payment_cpi_context,
            maker_amount,
            self.payment_mint.decimals,
        )?;

        let send_fee_accounts = TransferChecked {
            from: self.offer_vault_ata.to_account_info(),
            mint: self.payment_mint.to_account_info(),
            to: self.treasury_payment_ata.to_account_info(),
            authority: self.offer.to_account_info(),
        };

        let send_fee_cpi_context = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            send_fee_accounts,
            offer_signer,
        );

        // 2. Send fee to treasury
        transfer_checked(send_fee_cpi_context, fee, self.payment_mint.decimals)?;

        // 3. Close vault ATA — return rent to taker
        let close_vault_account = CloseAccount {
            account: self.offer_vault_ata.to_account_info(),
            destination: self.taker.to_account_info(),
            authority: self.offer.to_account_info(),
        };

        close_account(CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            close_vault_account,
            offer_signer,
        ))?;

        // 4. Transfer NFT from listing PDA to taker
        let listing_bump = self.listing.bump;
        let listing_seeds = &[LISTING, asset_key.as_ref(), &[listing_bump]];
        let listing_signer = &[&listing_seeds[..]];

        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(self.collection.as_deref().map(|c| c.as_ref()))
            .payer(&self.maker.to_account_info())
            .authority(Some(&self.listing.to_account_info()))
            .new_owner(&self.taker.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(listing_signer)?;

        // 5. Mint reward tokens to taker
        let marketplace_name = self.marketplace.name.clone();
        let marketplace_bump = self.marketplace.bump;
        let marketplace_seeds = &[
            MARKETPLACE,
            marketplace_name.as_bytes(),
            &[marketplace_bump],
        ];
        let marketplace_signer = &[&marketplace_seeds[..]];

        mint_to_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintToChecked {
                    mint: self.rewards_mint.to_account_info(),
                    to: self.taker_reward_ata.to_account_info(),
                    authority: self.marketplace.to_account_info(),
                },
                marketplace_signer,
            ),
            amount,
            MINT_DECIMALS,
        )
    }
}
