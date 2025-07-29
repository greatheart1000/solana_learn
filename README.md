# solana_learn

### CPI跨程序调用 SPL Token Program
https://www.seahorsecookbook.com/tutorial/basics/cpis
```
mint SPL Tokens to an account rust版本 
pub fn mint_tokens(
    ctx: Context<MintTokens>,
    amount: u64
) -> Result<()> {
    let cpi_accounts = MintTo {
        mint: ctx.accounts.token_mint.to_account_info(),
        to: ctx.accounts.recipient_account.to_account_info(),
        authority: ctx.accounts.mint_authority.to_account_info(),
    };


    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);


    token::mint_to(cpi_ctx, amount)?;


    Ok(())
}


#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(
        mut,
        mint::authority = mint_authority
    )]
    pub token_mint: Account<'info, Mint>,


    #[account(mut)]
    pub mint_authority: Signer<'info>,


    pub recipient_account: Account<'info, TokenAccount>,


    pub token_program: Program<'info, Token>,
}
python版本 
@instruction
def mint_tokens(
    token_mint: TokenMint,
    recipient: TokenAccount,
    mint_authority: Signer,
    amount: u64
):
    token_mint.mint(
        authority = mint_authority,
        to = recipient,
        amount = amount
    )

@instruction
def init_token_account(
    token_account: Empty[TokenAccount],
    token_mint: TokenMint,
    authority: Signer
):
    token_account.init(
      payer = authority,
      seeds = ['token-account', authority],
      mint = token_mint,
      authority = authority
    )


@instruction
def use_token_account(
    authority_account: TokenAccount,
    recipient: TokenAccount,
    authority: Signer,
    amount: u64
):
    authority_account.transfer(
      authority = authority,
      to = recipient,
      amount = amount
  )
```
### 使用 Seahorse 构建一个 SPL 水龙头（Faucet）程序
https://www.seahorsecookbook.com/tutorial/basics/milestone-project-faucet <br>
我们要构建一个水龙头（Faucet）程序，授权者（authority）可以创建水龙头来分发代币，用户则可以从这些水龙头中提取代币。首先，我们需要理清：  <br>

在账户中需要存储哪些数据？  <br>
需要实现哪些指令（instruction）？  <br>
#### 一、账户数据  <br>
Faucet 账户  <br>
存储与分发代币相关的数据：  <br>

mint（代币铸造地址）  <br>
decimals（小数位）  <br>
max_withdraw（最大提取额度阈值）  <br>
owner（水龙头账户的所有者）  <br>
Withdrawer 账户  <br>
存储与提取者相关的数据：  <br>

owner（提取者账户的所有者）  <br>
last_withdraw（上次提取的时间戳）  <br>
#### 二、指令（Instructions）  <br>
initialize_faucet：初始化一个水龙头账户  <br>
deposit：向水龙头中存入代币  <br>
initialize_withdrawer：初始化一个提取者账户  <br>
withdraw：从水龙头提取代币  <br>

好，让我们来看一下发生了什么。我们的 initialize_faucet 函数接受 6 个参数：  <br>

mint：要为其创建水龙头的代币铸造地址  <br>
faucet：一个空的水龙头账户  <br>
signer：用于签名该指令的授权者  <br>
faucet_account：一个空的 TokenAccount，用来存放水龙头的代币  <br>
decimals：代币的小数位  <br>
max_withdraw：每次提取的最大代币数量  <br>
首先，我们使用 .bump() 方法派生水龙头 PDA 的 bump，并以 signer 作为付费者、以 mint 作为 seeds 来初始化水龙头账户。  <br>
然后，初始化用于存放水龙头代币的 TokenAccount，同样以 mint 作为参数，并将该账户的 authority 设置为水龙头账户。 <br>
这样，当用户想从水龙头提取代币时，水龙头账户就可以对代币转账指令进行签名。 <br>

接着，我们为水龙头账户的各字段赋值。至此，initialize_faucet 指令就准备就绪了。  <br>
```
@instruction
def initialize_withdrawer(
  signer: Signer, 
  withdrawer: Empty[Withdrawer]
  ):
  withdrawer = withdrawer.init(
    payer = signer,
    seeds = ['withdrawer', signer]
  )
  withdrawer.owner = signer.key()
```
这个指令接收 signer 和一个空的 Withdrawer 账户。我们在第一步使用 signer 作为 seeds 初始化 withdrawer，然后将 signer 的公钥赋值给 withdrawer 的 owner 字段。

```
@instruction
def withdraw(
  mint: TokenMint, 
  withdrawer_account: TokenAccount, 
  faucet_account: TokenAccount, 
  faucet: Faucet,
  n: u64, 
  withdrawer: Withdrawer,
  signer: Signer,
  clock: Clock
):
  assert mint.key() == faucet.mint, 'The Token mint you are trying to withdraw does not match the faucet mint'
  assert signer.key() == withdrawer.owner, 'You have provided a wrong Withdrawer account'

  timestamp: i64 = clock.unix_timestamp()
  assert timestamp - 60 > withdrawer.last_withdraw, 'Your transaction has been rate limited, please try again in one minute'

  assert n <= faucet.max_withdraw, 'The maximal amount you can withdraw is exceeded.'

  amount = n * faucet.decimals
  bump = faucet.bump

  faucet_account.transfer(
    authority = faucet,
    to = withdrawer_account,
    amount = amount,
    signer = ['mint', mint, bump]
  )

  withdrawer.last_withdraw = timestamp

withdraw 是我们程序的核心部分。先来看一下各个参数：

mint：要提取的代币的铸造地址
withdrawer_account：提取者（Withdrawer）的代币账户
faucet_account：水龙头（Faucet）的代币账户
faucet：Faucet 账户的元数据
n：本次提取的代币数量
withdrawer：Withdrawer 账户的元数据
signer：交易签名者，这里是提取者本人
clock：Solana 的 Clock 系统账户，用于获取当前时间戳，实现速率限制
主要逻辑如下：

防止欺诈

检查 mint.key() 是否等于 faucet.mint；
检查 signer.key() 是否等于 withdrawer.owner。
速率限制

调用 clock.unix_timestamp() 获取当前时间戳 timestamp；
确保 timestamp - 60 > withdrawer.last_withdraw，否则报错：一分钟内不能重复提取。
数量限制

确保请求数量 n 不超过 faucet.max_withdraw。
计算实际转账数额
amount = n * faucet.decimals；
取出 PDA 的 bump = faucet.bump。
转账
```


