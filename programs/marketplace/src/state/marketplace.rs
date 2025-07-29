use anchor_lang::prelude::*; // Import essential Anchor framework components

#[account] // Tell Anchor this is an account that can be serialized/deserialized
#[derive(InitSpace)] // Automatically calculate space needed for this struct
pub struct Marketplace {    
    pub admin: Pubkey,              // Public key of marketplace administrator (32 bytes)
    pub fee: u16,                   // Marketplace fee in basis points (e.g., 200 = 2%) (2 bytes)
    pub bump: u8,                   // Canonical bump seed for marketplace PDA (1 byte)
    pub treasury_bump: u8,          // Canonical bump seed for treasury PDA (1 byte)
    pub rewards_bump: u8,           // Canonical bump seed for rewards mint PDA (1 byte)
    #[max_len(32)]  
    pub name: String,               // Marketplace name (max 32 chars) (4 + 32 bytes)
}