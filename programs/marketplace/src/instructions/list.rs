// Core Anchor framework imports
use anchor_lang::prelude::*;

// SPL Token program imports
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{Metadata, MetadataAccount},
    token::{transfer_checked, TransferChecked},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

// Local state imports
use crate::{Listing, Marketplace};

#[derive(Accounts)]  // Define accounts needed for listing instruction
pub struct List<'info> {
    #[account(mut)] // Mutable because seller pays for accounts and signs
    pub seller: Signer<'info>, // Person listing their NFT for sale

    // account which stores the listing details
    #[account(
        init, // Create new listing account
        payer = seller, // Seller pays rent for listing account
        space = 8 + Listing::INIT_SPACE, // Account size: discriminator + listing data
        seeds = [marketplace.key().as_ref(), seller_mint.key().as_ref()], // PDA: marketplace + NFT mint
        bump // Canonical bump for deterministic listing address
    )]
    pub listing: Account<'info, Listing>, // Store listing price and seller info

    // nft mint which is kept for sale in listing
    pub seller_mint: InterfaceAccount<'info, Mint>, // The NFT token mint being sold

    // account which is storing the nft
    #[account(
        associated_token::mint = seller_mint, // ATA for the specific NFT mint
        associated_token::authority = seller // Seller owns this token account
    )]
    pub seller_ata: InterfaceAccount<'info, TokenAccount>, // Seller's NFT token account

    // account whcih has the marketplace details
    #[account(
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes()], // PDA: "marketplace" + name
        bump // Verify this is the correct marketplace PDA
    )]
    pub marketplace: Account<'info, Marketplace>, // Read marketplace config (fees, admin, etc.)

    // account where the nft is kept in hold
    #[account(
        init, // Create vault to hold NFT during listing
        payer = seller, // Seller pays for vault creation
        associated_token::mint = seller_mint, // ATA for the NFT mint
        associated_token::authority = listing // Listing PDA controls the vault
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>, // Escrow account holding NFT

    // metadata account which is used to verify the nft
    pub collection_mint: InterfaceAccount<'info, Mint>, // Collection this NFT belongs to

    #[account(
        seeds = [ // Metaplex metadata PDA structure
            b"metadata", // Metaplex metadata seed
            metadata_program.key().as_ref(), // Metaplex program ID
            seller_mint.key().as_ref() // The NFT mint
        ],
        bump, // Canonical bump for metadata account
        seeds::program = metadata_program.key(), // Verify this PDA belongs to Metaplex
        constraint = metadata.collection.as_ref().unwrap().key.as_ref() == collection_mint.key().as_ref(), // Verify NFT belongs to specified collection
        constraint = metadata.collection.as_ref().unwrap().verified == true, // Verify collection is properly verified
    )]
    pub metadata: Account<'info, MetadataAccount>, // NFT metadata with collection info

    #[account(
        seeds=[ // Metaplex master edition PDA structure
            b"metadata", // Metaplex seed
            metadata_program.key().as_ref(), // Metaplex program ID
            seller_mint.key().as_ref(), // NFT mint
            b"edition" // Edition-specific seed
        ],
        bump, // Canonical bump for edition account
        seeds::program = metadata_program.key() // Verify PDA belongs to Metaplex
    )]
    pub edition: Account<'info, MetadataAccount>, // Master edition (proves it's unique NFT)

    // Program accounts
    pub metadata_program: Program<'info, Metadata>, // Metaplex metadata program
    pub system_program: Program<'info, System>, // For account creation
    pub token_program: Interface<'info, TokenInterface>, // For token operations
    pub associated_token_program: Program<'info, AssociatedToken>, // For ATA creation
}

impl<'info> List<'info> {
    pub fn list(&mut self, price: u64, bumps: &ListBumps) -> Result<()> {
        self.listing.set_inner(Listing { // Write listing data to account
            maker: self.seller.key(), // Who is selling the NFT
            maker_mint: self.seller_mint.key(), // Which NFT is being sold
            price, // Sale price in lamports
            bump: bumps.listing, // Store listing PDA bump
        });
        Ok(()) // Return success
    }

    pub fn deposit_nft(&mut self) -> Result<()> {
        let cpi_program = self.token_program.to_account_info(); // Get token program for CPI

        let cpi_accounts = TransferChecked { // Set up token transfer accounts
            from: self.seller_ata.to_account_info(), // Source: seller's token account
            to: self.vault.to_account_info(), // Destination: vault token account  
            authority: self.seller.to_account_info(), // Who authorizes the transfer
            mint: self.seller_mint.to_account_info(), // Which token mint to transfer
        };

        let ctx = CpiContext::new(cpi_program, cpi_accounts); // Create cross-program invocation context

        transfer_checked(ctx, 1, 0)?; // Transfer 1 NFT (amount=1, decimals=0 for NFTs)
        Ok(()) // Return success
    }
}