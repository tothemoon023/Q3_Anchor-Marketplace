// Core Anchor framework imports
use anchor_lang::prelude::*;

// SPL Token program imports
use anchor_spl::{
    token_2022::close_account,
    token_interface::{
        transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked,
    },
};

// Local state imports
use crate::{Listing, Marketplace};

#[derive(Accounts)] // Define accounts needed for delisting instruction
pub struct Delist<'info> {
    #[account(mut)] // Mutable because seller receives refunded account rent
    pub seller: Signer<'info>, // Original seller who wants to cancel their listing

    // Account which stores the listing details
    #[account(
        mut, // Mutable because we're closing this account
        close = seller, // When closing, send remaining lamports to seller
        seeds = [marketplace.key().as_ref(), mint.key().as_ref()], // PDA: marketplace + NFT mint
        constraint = listing.maker == seller.key(), // Verify this listing belongs to the seller
        bump = listing.bump // Use stored bump to verify PDA
    )]
    pub listing: Account<'info, Listing>, // The listing account to be closed

    // NFT mint which is kept for sale in listing
    pub mint: InterfaceAccount<'info, Mint>, // The NFT token mint being delisted

    // Account which is storing the NFT
    #[account(
        associated_token::mint = mint, // ATA for this specific NFT mint
        associated_token::authority = seller // Seller owns this token account
    )]
    pub seller_ata: InterfaceAccount<'info, TokenAccount>, // Seller's token account to receive NFT back

    // Account which has the marketplace details
    #[account(
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes()], // PDA: "marketplace" + name
        bump // Verify this is the correct marketplace PDA
    )]
    pub marketplace: Account<'info, Marketplace>, // Read marketplace configuration

    // Account where the NFT is kept in hold
    #[account(
        mut, // Mutable because we're closing this vault
        associated_token::mint = mint, // ATA for this NFT mint
        associated_token::authority = listing // Listing PDA controls this vault
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>, // Vault holding the NFT during listing

    // Program accounts
    pub system_program: Program<'info, System>, // For account operations
    pub token_program: Interface<'info, TokenInterface>, // For token transfers and closures
}

impl<'info> Delist<'info> {
    /// Withdraws the NFT from the marketplace vault back to the seller's token account
    /// Uses the listing PDA as authority to authorize the transfer
    pub fn withdraw_nft(&mut self) -> Result<()> {
        let cpi_program = self.token_program.to_account_info(); // Get token program for CPI

        let cpi_accounts = TransferChecked { // Set up NFT transfer back to seller
            from: self.vault.to_account_info(), // Source: vault holding the NFT
            to: self.seller_ata.to_account_info(), // Destination: seller's token account
            authority: self.listing.to_account_info(), // Listing PDA has authority over vault
            mint: self.mint.to_account_info(), // Which NFT mint to transfer
        };

        // Create PDA signing seeds for listing authority
        let seeds = &[
            &self.marketplace.key().to_bytes()[..], // Marketplace address as bytes
            &self.mint.key().to_bytes()[..], // NFT mint address as bytes
            &[self.listing.bump], // Listing bump seed as single-element array
        ];
        let signer_seeds = &[&seeds[..]]; // Create nested array structure for signing

        let ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds); // Create CPI context with PDA signing

        transfer_checked(ctx, 1, 0) // Transfer 1 NFT back to seller (amount=1, decimals=0 for NFTs)
    }

    /// Closes the marketplace vault account and transfers remaining lamports to seller
    /// This completes the delisting process and cleans up the vault
    pub fn close_account(&mut self) -> Result<()> {
        // Create PDA signing seeds for listing authority (same as withdraw_nft)
        let seeds = &[
            &self.marketplace.key().to_bytes()[..], // Marketplace address as bytes
            &self.mint.key().to_bytes()[..], // NFT mint address as bytes
            &[self.listing.bump], // Listing bump seed
        ];
        let signer_seeds = &[&seeds[..]]; // Nested array for PDA signing

        let cpi_accounts = CloseAccount { // Set up vault closure accounts
            account: self.vault.to_account_info(), // Vault account to close
            authority: self.listing.to_account_info(), // Listing PDA has authority
            destination: self.seller.to_account_info(), // Send remaining lamports to seller
        };

        let ctx = CpiContext::new_with_signer( // Create CPI context with signing
            self.token_program.to_account_info(), // Token program for vault closure
            cpi_accounts, // Accounts for closure
            signer_seeds, // PDA signing seeds
        );

        close_account(ctx) // Close the vault and send lamports to seller
    }
}