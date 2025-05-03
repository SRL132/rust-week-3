use anchor_lang::prelude::*;
use anchor_spl::token::{ self, Mint, Token, TokenAccount, Transfer};
use crate::errors::ErrorCode;
use crate::{stake_pool_signer_seeds, state::StakePool };  
  
#[derive(Accounts)] 
pub struct Slashing<'info> {
    // Payer to actually stake the mint tokens
    #[account(mut)]
    pub authority: Signer<'info>,  

    /// Vault of the StakePool token will be transfer to
    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub stake_mint: Account<'info, Mint>,

    /// StakePool owning the vault that holds the deposit
    #[account(
        mut,
        has_one = vault @ ErrorCode::InvalidStakePoolVault,
        has_one = stake_mint @ ErrorCode::InvalidAuthority,
    )]
    pub stake_pool: AccountLoader<'info, StakePool>,
    pub token_program: Program<'info, Token>,
}
 //@audit why are is_locker arbitrary? and what is the purpose of router?
pub fn slashing_handler<'info>(
    ctx: Context<Slashing>,
    amount: u64,
    router: u8,
    is_locked: u8 
) -> Result<()> {
    {    
        let stake_pool = &mut ctx.accounts.stake_pool.load_mut()?;
        let pool = &mut stake_pool.reward_pools[usize::from(router)];
        pool.is_locked = is_locked;

        let cpi_ctx = CpiContext {
            program: ctx.accounts.token_program.to_account_info(),
            accounts: Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.stake_pool.to_account_info(),
            },
            remaining_accounts: Vec::new(),
            //@audit is the seed public?
            signer_seeds: &[stake_pool_signer_seeds!(stake_pool)],
        };
        let _ = token::transfer(cpi_ctx, amount);

        Ok(())
    } 
}