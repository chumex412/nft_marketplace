import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Commitment,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { expect } from "chai";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
  mplCore,
  createCollectionV1,
  createV1,
  transferV1,
  fetchAssetV1,
  MPL_CORE_PROGRAM_ID,
} from "@metaplex-foundation/mpl-core";
import {
  keypairIdentity,
  generateSigner,
  publicKey,
} from "@metaplex-foundation/umi";
import {
  fromWeb3JsKeypair,
  toWeb3JsPublicKey,
} from "@metaplex-foundation/umi-web3js-adapters";

import { NftMarketplace } from "../target/types/nft_marketplace";

const commitment: Commitment = "confirmed";

const MARKETPLACE_NAME = "TestMarket";
const MARKETPLACE_FEE = 250; // 2.5% in basis points
const LISTING_PRICE = new anchor.BN(1 * LAMPORTS_PER_SOL);

describe("nft_marketplace", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const confirmTxn = async (signature: string) => {
    const latestBlockHash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction(
      { signature, ...latestBlockHash },
      commitment,
    );
  };

  const program = anchor.workspace.NftMarketplace as Program<NftMarketplace>;
  const admin = provider.wallet;

  const maker = Keypair.generate();
  const taker = Keypair.generate();

  // UMI instance for MPL Core NFT creation (maker signs)
  const umi = createUmi(provider.connection.rpcEndpoint, "confirmed").use(mplCore());
  const collectionSigner = generateSigner(umi);
  const assetSigner = generateSigner(umi);

  const collectionPubkey = toWeb3JsPublicKey(collectionSigner.publicKey);
  const assetPubkey = toWeb3JsPublicKey(assetSigner.publicKey);
  const mplCoreProgramId = toWeb3JsPublicKey(MPL_CORE_PROGRAM_ID);

  // PDAs
  const [marketplacePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("marketplace"), Buffer.from(MARKETPLACE_NAME)],
    program.programId,
  );

  const [treasuryPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("treasury"), marketplacePda.toBuffer()],
    program.programId,
  );

  const [rewardsMintPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("rewards"), marketplacePda.toBuffer()],
    program.programId,
  );

  const [listingPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("listing"), assetPubkey.toBuffer()],
    program.programId,
  );

  const takerRewardAta = getAssociatedTokenAddressSync(
    rewardsMintPda,
    taker.publicKey,
  );

  before(async () => {
    // Fund maker and taker for marketplace operations
    const [makerSig, takerSig] = await Promise.all([
      provider.connection.requestAirdrop(
        maker.publicKey,
        10 * LAMPORTS_PER_SOL,
      ),
      provider.connection.requestAirdrop(
        taker.publicKey,
        10 * LAMPORTS_PER_SOL,
      ),
    ]);
    await Promise.all([confirmTxn(makerSig), confirmTxn(takerSig)]);

    umi.use(keypairIdentity(fromWeb3JsKeypair(admin.payer ?? maker)));

    await createCollectionV1(umi, {
      collection: collectionSigner,
      name: "Test Collection",
      uri: "https://example.com/collection",
    }).sendAndConfirm(umi);

    await createV1(umi, {
      asset: assetSigner,
      collection: collectionSigner.publicKey,
      name: "Test NFT #1",
      uri: "https://example.com/nft/1",
    }).sendAndConfirm(umi);

    // Transfer the asset to maker so they can list it on the marketplace
    await transferV1(umi, {
      asset: assetSigner.publicKey,
      collection: collectionSigner.publicKey,
      newOwner: publicKey(maker.publicKey.toBase58()),
    }).sendAndConfirm(umi);
  });

  it("Verifies the NFT was minted to maker", async () => {
    const asset = await fetchAssetV1(umi, assetSigner.publicKey);
    expect(asset.owner.toString()).to.eq(maker.publicKey.toString());
  });

  it("Initializes the marketplace", async () => {
    const tx = await program.methods
      .initialize(MARKETPLACE_NAME, MARKETPLACE_FEE)
      .accountsStrict({
        admin: admin.publicKey,
        marketplace: marketplacePda,
        treasury: treasuryPda,
        rewardsMint: rewardsMintPda,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    await confirmTxn(tx);

    const marketplace = await program.account.marketPlace.fetch(marketplacePda);
    expect(marketplace.admin.toString()).to.eq(admin.publicKey.toString());
    expect(marketplace.fee).to.eq(MARKETPLACE_FEE);
    expect(marketplace.name).to.eq(MARKETPLACE_NAME);
  });

  it("Fails to initialize with an empty name", async () => {
    const [badMarketplace] = PublicKey.findProgramAddressSync(
      [Buffer.from("marketplace"), Buffer.from("  ")],
      program.programId,
    );
    const [badTreasury] = PublicKey.findProgramAddressSync(
      [Buffer.from("treasury"), badMarketplace.toBuffer()],
      program.programId,
    );
    const [badRewardsMint] = PublicKey.findProgramAddressSync(
      [Buffer.from("rewards"), badMarketplace.toBuffer()],
      program.programId,
    );

    try {
      await program.methods
        .initialize("  ", MARKETPLACE_FEE)
        .accountsStrict({
          admin: admin.publicKey,
          marketplace: badMarketplace,
          treasury: badTreasury,
          rewardsMint: badRewardsMint,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
      expect.fail("Expected initialization to fail with empty name");
    } catch (err) {
      expect(err).to.be.instanceOf(Error);
    }
  });

  it("Lists the NFT", async () => {
    const makerBalanceBefore = await provider.connection.getBalance(
      maker.publicKey,
    );

    const tx = await program.methods
      .list(LISTING_PRICE)
      .accountsStrict({
        maker: maker.publicKey,
        asset: assetPubkey,
        collection: collectionPubkey,
        listing: listingPda,
        paymentMint: null,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: mplCoreProgramId,
      })
      .signers([maker])
      .rpc();

    await confirmTxn(tx);

    // Listing account has correct data
    const listing = await program.account.listing.fetch(listingPda);
    expect(listing.maker.toString()).to.eq(maker.publicKey.toString());
    expect(listing.asset.toString()).to.eq(assetPubkey.toString());
    expect(listing.price.toNumber()).to.eq(LISTING_PRICE.toNumber());

    // NFT is now owned by the listing PDA (locked in escrow)
    const asset = await fetchAssetV1(umi, assetSigner.publicKey);
    expect(asset.owner.toString()).to.eq(listingPda.toString());

    // Maker paid rent for the listing PDA
    const makerBalanceAfter = await provider.connection.getBalance(
      maker.publicKey,
    );
    expect(makerBalanceAfter).to.be.lessThan(makerBalanceBefore);
  });

  it("Fails to list an already-listed NFT", async () => {
    try {
      await program.methods
        .list(LISTING_PRICE)
        .accountsStrict({
          maker: maker.publicKey,
          asset: assetPubkey,
          collection: collectionPubkey,
          listing: listingPda,
          paymentMint: null,
          systemProgram: SystemProgram.programId,
          mplCoreProgram: mplCoreProgramId,
        })
        .signers([maker])
        .rpc();
      expect.fail("Expected listing to fail on already-listed NFT");
    } catch (err) {
      expect(err).to.be.instanceOf(Error);
    }
  });

  it("Buys the listed NFT", async () => {
    const makerBalanceBefore = await provider.connection.getBalance(
      maker.publicKey,
    );
    const treasuryBalanceBefore = await provider.connection.getBalance(
      treasuryPda,
    );

    const tx = await program.methods
      .buy()
      .accountsStrict({
        taker: taker.publicKey,
        maker: maker.publicKey,
        marketplace: marketplacePda,
        asset: assetPubkey,
        collection: collectionPubkey,
        listing: listingPda,
        treasury: treasuryPda,
        rewardsMint: rewardsMintPda,
        takerRewardAta,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        mplCoreProgram: mplCoreProgramId,
      })
      .signers([taker])
      .rpc();

    await confirmTxn(tx);

    // NFT transferred to taker
    const asset = await fetchAssetV1(umi, assetSigner.publicKey);
    expect(asset.owner.toString()).to.eq(taker.publicKey.toString());

    // Listing account is closed (rent returned to maker)
    const listingAccount = await provider.connection.getAccountInfo(listingPda);
    expect(listingAccount).to.be.null;

    // Maker received (price - fee) + listing rent refund
    const makerBalanceAfter = await provider.connection.getBalance(
      maker.publicKey,
    );
    expect(makerBalanceAfter).to.be.greaterThan(makerBalanceBefore);

    // Treasury received the fee
    const treasuryBalanceAfter = await provider.connection.getBalance(
      treasuryPda,
    );
    expect(treasuryBalanceAfter).to.be.greaterThan(treasuryBalanceBefore);

    // Taker reward ATA was initialized
    const rewardAtaInfo = await provider.connection.getAccountInfo(
      takerRewardAta,
    );
    expect(rewardAtaInfo).to.not.be.null;
  });

  it("Fails to buy a closed listing", async () => {
    try {
      await program.methods
        .buy()
        .accountsStrict({
          taker: taker.publicKey,
          maker: maker.publicKey,
          marketplace: marketplacePda,
          asset: assetPubkey,
          collection: collectionPubkey,
          listing: listingPda,
          treasury: treasuryPda,
          rewardsMint: rewardsMintPda,
          takerRewardAta,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          mplCoreProgram: mplCoreProgramId,
        })
        .signers([taker])
        .rpc();
      expect.fail("Expected buy to fail on a closed listing");
    } catch (err) {
      expect(err).to.be.instanceOf(Error);
    }
  });
});
