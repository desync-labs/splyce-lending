use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer};

pub fn transfer_token_from<'a>(
    token_program: AccountInfo<'a>,
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    amount: u64,
    seeds: &[&[u8]],
) -> Result<()> {
    token::transfer(
        CpiContext::new_with_signer(
            token_program,
            Transfer {
                from,
                to,
                authority,
            },
            &[&seeds]
        ),
        amount,
    )
}

pub fn transfer_token_to<'a>(
    token_program: AccountInfo<'a>,
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    amount: u64,
) -> Result<()> {
    token::transfer(
        CpiContext::new(
            token_program,
            Transfer {
                from,
                to,
                authority,
            }
        ),
        amount,
    )
}

pub fn mint_tokens<'a>(
    token_program: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    to: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    if !signer_seeds.is_empty() {
        // When using a PDA (Program Derived Address) as the authority
        token::mint_to(
            CpiContext::new_with_signer(
                token_program,
                MintTo {
                    mint,
                    to,
                    authority,
                },
                signer_seeds, // Correctly passing signer seeds
            ),
            amount,
        )
    } else {
        // When the authority is a signer (private key holder)
        token::mint_to(
            CpiContext::new(
                token_program,
                MintTo {
                    mint,
                    to,
                    authority,
                },
            ),
            amount,
        )
    }
}