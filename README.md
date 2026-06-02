# NFT Marketplace

A Solana program built with Anchor that enables listing an asset in a marketplace in exchange for SPL token rewards.

## Instructions

| Instruction  | Description                                   |
| ------------ | --------------------------------------------- |
| `initialize` | Initializes the marketplace                   |
| `list`       | List the assets on the marketplace collection |
| `buy`        | Buy asset on-chain                            |
| `make_offer` | Make offer get an asset for a price           |
| `buy_offer`  | Buy assets based on offer                     |

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)
- [Anchor CLI](https://www.anchor-lang.com/docs/installation)
- [TypeScript](https://www.typescriptlang.org/)
- Node.js + Yarn

## Setup & Test

```bash
yarn install
anchor build
anchor test
```

## Screenshot

![Screenshot of the result of anchor test](https://res.cloudinary.com/da8vqkdmt/image/upload/v1780391504/Screen_Shot_2026-06-02_at_10.11.02_AM_kp8lmc.png)
