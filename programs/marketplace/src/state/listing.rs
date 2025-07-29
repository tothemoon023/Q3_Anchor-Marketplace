use anchor_lang::prelude::*; // Import essential Anchor framework components

#[account] // Tell Anchor this is an account that can be serialized/deserialized
#[derive(InitSpace)] // Automatically calculate space needed for this struct
pub struct Listing {
    pub maker: Pubkey,         // Public key of NFT seller (32 bytes)
    pub maker_mint: Pubkey,           // Public key of NFT mint being sold (32 bytes)
    pub price: u64,             // Sale price in lamports (8 bytes)
    pub bump: u8               // Canonical bump seed for listing PDA (1 byte)
}