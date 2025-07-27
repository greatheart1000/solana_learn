// 引入Anchor，以及SPL Token代币模块
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};

// 声明程序地址，可不填，编译时帮填
declare_id!("HJUWSUMgK2bDaY7Ve4xQZ46tcgfJQFSRWBMK1wTCaz6Z");

// Anchor的程序宏
#[program]
// 定义具体的模块
mod token_vault {

    // 导入当前模块的父模块的全部公开项（变量 、函数等）
    use super::*;

    // 初始化相关的账户信息
    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    // 令牌转入 函数
    pub fn transfer_in(ctx: Context<TransferAccounts>, amount: u64) -> Result<()> {
        msg!("Token amount transfer in: {}!", amount);

        // 构建转移令牌的指令
        let transfer_instruction = Transfer {
            // 个人账户的令牌ATA账户
            from: ctx.accounts.sender_token_account.to_account_info(),
            // 存储令牌的新账户
            to: ctx.accounts.vault_token_account.to_account_info(),
            // 签名者
            authority: ctx.accounts.signer.to_account_info(),
        };
    
        // 构建 跨程序调用指令（CPI）
        let cpi_ctx = CpiContext::new(
            // 调用目标是 SPL令牌程序
            ctx.accounts.token_program.to_account_info(),
            // 转移令牌指令
            transfer_instruction,

            // 因为转入的操作是由用户直接发起的，用户已经签名了整个交易
            // 由用户的 ATA 转入 存储令牌 的新账户，所以这里不需要再签名
        );

        // 执行转移指令
        anchor_spl::token::transfer(cpi_ctx, amount)?;
    
        Ok(())
    }

    // 令牌转出 函数
    pub fn transfer_out(ctx: Context<TransferAccounts>, amount: u64) -> Result<()> {
        msg!("Token amount transfer out: {}!", amount);

        // 构建转移令牌指令
        let transfer_instruction = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.sender_token_account.to_account_info(),
            authority: ctx.accounts.token_account_owner_pda.to_account_info(),
        };

        // 获取与 token_account_owner_pda PDA账户相关的 bump随机数
        let bump = ctx.bumps.token_account_owner_pda;
        // 生成 token_account_owner_pda PDA账户的 种子
        let seeds = &[b"token_account_owner_pda".as_ref(), &[bump]];
        // 创建签名者数组，seeds 来签名
        let signer = &[&seeds[..]];

        // 构建 跨程序调用（CPI）
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,

            // 这里是由 存储令牌的新账户，转出至 用户的 ATA 账户
            // 控制 存储令牌的新账户 是 管理PDA，所以这里需要添加一个签名者（管理 PDA）
            signer,
        );

        anchor_spl::token::transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {

    // 管理PDA账户
    // 作为 vault_token_account 的授权者
    #[account(
        // 如果PDA不存在，就创建它
        init_if_needed,
        // 创建PDA账户的费用由 signer 支付
        payer = signer,
        // 种子
        seeds=[b"token_account_owner_pda"],
        // 随机数 保证PDA地址的唯一 且不能与任何私钥关联，0~255范围内，从255递减尝试
        // 可以通过 findProgramAddress 生成
        bump,
        // 创建账户时指定分配的字节数，用来存储数据，并以此为存储空间支付租金
        space = 8
    )]
    // token_account_owner_pda 由智能合约控制，管理 vault_token_account
    // 因为是PDA账户，合约在需要时可以直接通过种子短语找到账户，无需私钥签名
    token_account_owner_pda: AccountInfo<'info>,

    // 存储令牌的新账户，并由 token_account_owner_pda 控制
    #[account(
        init_if_needed,
        payer = signer,
        // 种子 包含"token_vault"，以及mint_of_token_being_sent的公钥
        seeds=[b"token_vault", mint_of_token_being_sent.key().as_ref()],
        // 指定存储的令牌
        token::mint=mint_of_token_being_sent,
        // 指定该账户的所有者，即token_account_owner_pda
        // 不主动声明的话，默认为创建它的智能合约
        token::authority=token_account_owner_pda,
        bump
    )]
    vault_token_account: Account<'info, TokenAccount>,

    // 令牌的mint地址
    mint_of_token_being_sent: Account<'info, Mint>,

    // 签名者
    #[account(mut)]
    signer: Signer<'info>,

    // Solana 系统账户，自动生成
    system_program: Program<'info, System>,

    // SPL Token程序账户，自动生成
    token_program: Program<'info, Token>,

    // 计算租金相关的功能，自动生成
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct TransferAccounts<'info> {

    // 管理PDA
    #[account(mut,
        seeds=[b"token_account_owner_pda"],
        bump
    )]
    token_account_owner_pda: AccountInfo<'info>,

    // 存储令牌的新地址
    #[account(mut,
        seeds=[b"token_vault", mint_of_token_being_sent.key().as_ref()],
        bump,
        token::mint=mint_of_token_being_sent,
        token::authority=token_account_owner_pda,
    )]
    vault_token_account: Account<'info, TokenAccount>,

    // 发送者的令牌账户ATA
    #[account(mut)]
    sender_token_account: Account<'info, TokenAccount>,

    // 代币的mint地址
    mint_of_token_being_sent: Account<'info, Mint>,

    // 签名者
    #[account(mut)]
    signer: Signer<'info>,
    
    // 系统账户
    system_program: Program<'info, System>,

    // SPL 令牌程序
    token_program: Program<'info, Token>,

    // 计算租金相关的功能
    rent: Sysvar<'info, Rent>,
}
