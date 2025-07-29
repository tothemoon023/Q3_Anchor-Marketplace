// Core Anchor framework imports
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

// SPL Token program imports
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::transfer_checked,
    token_interface::{
        close_account, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked,
    },
};

// Local state and error imports
use crate::{Listing, Marketplace};
use crate::error::MarketplaceError;

#[derive(Accounts)]
pub struct Purchase<'info> {
    #[account(mut)] // Mutable because buyer pays for transaction fees and token account creation
    pub buyer: Signer<'info>, // Person purchasing the NFT from the marketplace

    #[account(
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes()], // PDA: "marketplace" + name
        bump // Verify this is the correct marketplace PDA
    )]
    pub marketplace: Account<'info, Marketplace>, // Marketplace configuration (fees, admin, etc.)

    #[account(mut)] // Mutable because seller receives payment
    pub seller: SystemAccount<'info>, // Original NFT seller who receives payment

    pub seller_mint: InterfaceAccount<'info, Mint>, // NFT mint being purchased

    // Account which holds the details about the listing
    #[account(
        seeds = [marketplace.key().as_ref(), seller_mint.key().as_ref()], // PDA: marketplace + NFT mint
        bump = listing.bump, // Use stored bump to verify listing PDA
        constraint = listing.maker == seller.key(), // Verify this listing belongs to the seller for security
    )]
    pub listing: Account<'info, Listing>, // Listing data (price, seller, etc.)

    // Buyer's token account for receiving the NFT
    #[account(
        init_if_needed, // Create buyer's ATA if it doesn't exist
        payer = buyer, // Buyer pays for account creation
        associated_token::mint = seller_mint, // ATA for this specific NFT mint
        associated_token::authority = buyer // Buyer owns this token account
    )]
    pub buyer_ata: InterfaceAccount<'info, TokenAccount>, // Buyer's token account to receive NFT

    // Treasury account where marketplace fees are collected
    #[account(
        mut, // Mutable because treasury receives fee payments
        seeds = [b"treasury", marketplace.key().as_ref()], // PDA: "treasury" + marketplace
        bump // Verify this is the correct treasury PDA
    )]
    pub treasury: SystemAccount<'info>, // Treasury account for marketplace fees

    // Vault account holding the NFT during the listing period
    #[account(
        mut, // Mutable because we're closing this vault after transfer
        associated_token::mint = seller_mint, // ATA for the NFT mint
        associated_token::authority = listing // Listing PDA controls the vault
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>, // Escrow vault holding the NFT

    // Program accounts
    pub system_program: Program<'info, System>, // For SOL transfers and account operations
    pub associated_token_program: Program<'info, AssociatedToken>, // For ATA creation
    pub token_program: Interface<'info, TokenInterface>, // For token operations
}

impl<'info> Purchase<'info> {
    /// Transfers SOL payment from buyer to seller and marketplace treasury
    /// Calculates marketplace fee and ensures proper payment distribution
    pub fn transfer_amounts(&mut self) -> Result<()> {
        // Validate fee percentage is reasonable (max 50% to prevent abuse)
        require!(
            self.marketplace.fee <= 5000, // Max 50% fee (5000 basis points)
            MarketplaceError::FeeTooHigh
        );

        // Calculate marketplace fee using safe arithmetic to prevent overflow
        let fees = (self.marketplace.fee as u64)
            .checked_mul(self.listing.price)
            .ok_or(MarketplaceError::MathOverflow)? // Handle multiplication overflow
            .checked_div(10000) // Divide by 10000 for basis points (1 basis point = 0.01%)
            .ok_or(MarketplaceError::MathOverflow)?; // Handle division overflow

        // Calculate amount seller receives after marketplace fee
        let seller_amount = self.listing.price
            .checked_sub(fees)
            .ok_or(MarketplaceError::MathOverflow)?; // Handle subtraction overflow

        // Transfer payment to seller (listing price minus marketplace fee)
        let seller_transfer_ctx = CpiContext::new(
            self.system_program.to_account_info(),
            Transfer {
                from: self.buyer.to_account_info(),
                to: self.seller.to_account_info(),
            },
        );
        transfer(seller_transfer_ctx, seller_amount)?;

        // Transfer marketplace fee to treasury
        let treasury_transfer_ctx = CpiContext::new(
            self.system_program.to_account_info(),
            Transfer {
                from: self.buyer.to_account_info(),
                to: self.treasury.to_account_info(),
            },
        );
        transfer(treasury_transfer_ctx, fees)?;

        Ok(())
    }

    /// Transfers the NFT from the marketplace vault to the buyer's token account
    /// Uses the listing PDA as authority to authorize the transfer
    pub fn transfer_nft(&mut self) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = TransferChecked {
            from: self.vault.to_account_info(), // Source: marketplace vault
            to: self.buyer_ata.to_account_info(), // Destination: buyer's token account
            authority: self.listing.to_account_info(), // Listing PDA has authority over vault
            mint: self.seller_mint.to_account_info(), // NFT mint being transferred
        };

        // Create PDA signing seeds for listing authority
        let seeds = &[
            &self.marketplace.key().to_bytes()[..], // Marketplace address as bytes
            &self.seller_mint.key().to_bytes()[..], // NFT mint address as bytes
            &[self.listing.bump], // Listing bump seed
        ];
        let signer_seeds = &[&seeds[..]]; // Nested array structure for PDA signing

        let ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        // Transfer 1 NFT (amount=1, decimals=0 for NFTs)
        transfer_checked(ctx, 1, 0)
    }

    /// Closes the empty vault account after NFT transfer
    /// Returns remaining lamports to the seller as compensation
    pub fn close_vault(&mut self) -> Result<()> {
        // Create PDA signing seeds for listing authority (same as transfer_nft)
        let seeds = &[
            &self.marketplace.key().to_bytes()[..], // Marketplace address as bytes
            &self.seller_mint.key().to_bytes()[..], // NFT mint address as bytes
            &[self.listing.bump], // Listing bump seed
        ];
        let signer_seeds = &[&seeds[..]]; // Nested array for PDA signing

        let cpi_accounts = CloseAccount {
            account: self.vault.to_account_info(), // Vault account to close
            authority: self.listing.to_account_info(), // Listing PDA has authority
            destination: self.seller.to_account_info(), // Send remaining lamports to seller
        };

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );

        close_account(ctx) // Close vault and transfer remaining lamports to seller
    }
}