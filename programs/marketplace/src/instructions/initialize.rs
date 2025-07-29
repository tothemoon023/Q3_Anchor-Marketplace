// Core Anchor framework imports
use anchor_lang::prelude::*;

// SPL Token program imports
use anchor_spl::{token::Token, token_interface::Mint};

// Local state imports
use crate::Marketplace;

#[derive(Accounts)] // Tell Anchor this struct defines instruction accounts
#[instruction(name:String)] // Access the 'name' parameter in account constraints
pub struct Initialize<'info> {
    // The admin is the person who creates the marketplace
    #[account(mut)] // Mark as mutable because admin pays for account creation
    pub admin: Signer<'info>, // Require admin signature to prevent unauthorized access

    // Main account which holds the every information about the marketplace
    // It is initialized by the admin and holds the admin's public key, fee, and
    // the bump seeds for the marketplace, treasury, and reward mint.
    // The name is used to identify the marketplace and is used as a seed for the PDA
    #[account(
        init, // Create this account (doesn't exist yet)
        payer = admin, // Admin pays the rent for this account
        seeds = [b"marketplace", name.as_str().as_bytes()], // PDA derived from "marketplace" + name
        bump, // Use canonical bump seed for deterministic address
        space = 8 + Marketplace::INIT_SPACE // 8 bytes discriminator + custom data size
    )]
    pub marketplace: Account<'info, Marketplace>, // Store marketplace config data

    // The treasury is a system account that holds the fees collected from the marketplace
    #[account(
        seeds = [b"treasury", marketplace.key().as_ref()], // PDA derived from "treasury" + marketplace address
        bump // Use canonical bump for deterministic treasury address
    )]
    pub treasury: SystemAccount<'info>, // System account (just holds SOL, no initialization needed)

    #[account(
        init, // Create new mint account
        payer = admin, // Admin pays for mint creation
        seeds = [b"reward", marketplace.key().as_ref()], // PDA derived from "reward" + marketplace
        bump, // Canonical bump for reward mint address
        mint::decimals = 6, // Set mint to 6 decimal places (like USDC)
        mint::authority = admin // Admin controls minting new reward tokens
    )]
    pub reward_mint: InterfaceAccount<'info, Mint>, // Token mint for marketplace rewards

    // Program accounts
    pub system_program: Program<'info, System>, // Needed for account creation
    pub token_program: Program<'info, Token>, // Needed for mint creation
}

impl<'info> Initialize<'info> {
    /// Initializes a new marketplace with the provided name and fee structure
    /// Sets up all the core marketplace configuration data
    pub fn init(&mut self, name: String, fee: u16, bumps: &InitializeBumps) -> Result<()> {
        // Create an instance of the Marketplace struct and initialize with provided parameters
        self.marketplace.set_inner(Marketplace { // Write data to the marketplace account
            admin: self.admin.key(), // Store admin's public key
            fee, // Store marketplace fee (basis points)
            bump: bumps.marketplace, // Store marketplace PDA bump
            treasury_bump: bumps.treasury, // Store treasury PDA bump
            rewards_bump: bumps.reward_mint, // Store reward mint PDA bump
            name, // Store marketplace name
        });
        Ok(()) // Return success
    }
}