#![allow(unexpected_cfgs)] // Suppress warnings about unexpected configuration flags
#![allow(deprecated)] // Suppress warnings about deprecated features

pub mod constants; // Module containing program constants
pub mod error; // Module containing custom error definitions
pub mod instructions; // Module containing all instruction handlers
pub mod state; // Module containing account state structures

use anchor_lang::prelude::*; // Import essential Anchor framework components

pub use constants::*; // Re-export all constants for easy access
pub use instructions::*; // Re-export all instruction structs
pub use state::*;  // Re-export all state structs

declare_id!("Ho3jBBGmZntNnvaYcsM6AbyjDbTKzrC6eK4vfRxyoBHS");

// #[program] macro tells Anchor this module contains the program's instruction handlers
// Each public function in this module becomes a callable instruction
// These are the entry points that users/clients can invoke
#[program]
pub mod marketplace {
    use super::*; // Import all the modules and types declared above

    // ========================================================================
    // INITIALIZE MARKETPLACE INSTRUCTION
    // ========================================================================
    // Creates a new marketplace with a name and fee structure
    // Only needs to be called once per marketplace instance
    //
    // Parameters:
    // - ctx: Contains all accounts needed for initialization
    // - name: Human-readable name for the marketplace (e.g., "SuperNFT Market")
    // - fee: Percentage fee charged on sales (in basis points, e.g., 250 = 2.5%)
    //
    // Returns: Result indicating success or failure
    // ========================================================================
    pub fn init_marketplace(ctx: Context<Initialize>, name: String, fee: u16) -> Result<()> {
        // Delegate to the Initialize struct's init method
        // ctx.accounts contains all the accounts defined in the Initialize struct
        // &ctx.bumps contains the bump seeds for any PDAs created
        ctx.accounts.init(name, fee, &ctx.bumps)
    }

    // ========================================================================
    // LIST NFT INSTRUCTION
    // ========================================================================
    // Allows an NFT holder to list their NFT for sale on the marketplace
    // The NFT is transferred to a vault controlled by the listing PDA
    //
    // Parameters:
    // - ctx: Contains seller account, NFT mint, marketplace, vault, etc.
    // - price: Sale price in lamports (1 SOL = 1,000,000,000 lamports)
    //
    // Returns: Result indicating success or failure
    // ========================================================================
    pub fn listing(ctx: Context<List>, price: u64) -> Result<()> {
        // First, create the listing account with price and seller information
        // This must succeed before transferring the NFT to ensure atomicity
        ctx.accounts.list(price, &ctx.bumps)?; // ? operator propagates errors

        // Then transfer the NFT from seller to the marketplace vault
        // The vault is controlled by the listing PDA for security
        ctx.accounts.deposit_nft()
    }

    // ========================================================================
    // DELIST NFT INSTRUCTION
    // ========================================================================
    // Allows the original seller to cancel their listing and get their NFT back
    // This closes the listing account and vault, refunding rent to the seller
    //
    // Parameters:
    // - ctx: Contains seller account, listing, vault, marketplace, etc.
    //
    // Returns: Result indicating success or failure
    // ========================================================================
    pub fn delisting(ctx: Context<Delist>) -> Result<()> {
        // First, transfer the NFT back from vault to seller's token account
        // Must happen before closing accounts to avoid losing the NFT
        ctx.accounts.withdraw_nft()?; // ? operator propagates errors

        // Then close the vault account, sending remaining lamports to seller
        // This cleans up the marketplace and refunds rent
        ctx.accounts.close_account()
    }

    // ========================================================================
    // PURCHASE NFT INSTRUCTION
    // ========================================================================
    // Allows a buyer to purchase a listed NFT by paying the asking price
    // Handles payment distribution and NFT transfer in a single atomic transaction
    //
    // Parameters:
    // - ctx: Contains buyer, seller, marketplace, listing, vault, etc.
    //
    // Returns: Result indicating success or failure
    // ========================================================================
    pub fn purchase(ctx: Context<Purchase>) -> Result<()> {
        // First, handle all payment transfers (buyer -> seller, marketplace fee)
        // Payment must be completed before NFT transfer for security
        ctx.accounts.transfer_amounts()?; // ? operator propagates errors

        // Then transfer the NFT from vault to buyer's token account
        // Buyer now owns the NFT after successful payment
        ctx.accounts.transfer_nft()?; // ? operator propagates errors

        // Finally, close the empty vault account to clean up and refund rent
        // This completes the purchase and cleans up marketplace state
        ctx.accounts.close_vault()
    }
}