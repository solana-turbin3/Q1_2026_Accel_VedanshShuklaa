import * as anchor from "@coral-xyz/anchor";
import { init as initTuktuk, taskQueueAuthorityKey } from "@helium/tuktuk-sdk";
import { PublicKey, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";
import { sendInstructions } from "@helium/spl-utils";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const wallet = provider.wallet as anchor.Wallet;

  console.log("Setting up task queue for automated fee collector...");
  console.log("Wallet:", wallet.publicKey.toBase58());

  const programId = new PublicKey("8Axod2sBdMn6b7Nwicbtujwj6Gsv3wfBh6NjDtPPVKAh");
  
  // Your queue authority PDA
  const [queueAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from("queue_authority")],
    programId
  );
  console.log("Queue Authority PDA:", queueAuthority.toBase58());

  // Initialize TukTuk
  const tuktukProgram = await initTuktuk(provider);

  // Use an existing task queue or create one using tuktuk-cli:
  // tuktuk -u <rpc-url> -w <wallet-path> task-queue create --name "fee-collector-queue" --capacity 100
  const taskQueue = new PublicKey("YOUR_TASK_QUEUE_ADDRESS");

  // Add the program's queue_authority PDA as an authorized queue authority
  const taskQueueAuthorityPda = taskQueueAuthorityKey(taskQueue, queueAuthority)[0];
  const taskQueueAuthorityInfo = await provider.connection.getAccountInfo(taskQueueAuthorityPda);

  if (!taskQueueAuthorityInfo) {
    console.log("Adding queue authority for program PDA...");
    await tuktukProgram.methods
      .addQueueAuthorityV0()
      .accounts({
        payer: wallet.publicKey,
        queueAuthority: queueAuthority,
        taskQueue: taskQueue,
      })
      .rpc({ skipPreflight: true });
    console.log("Queue authority added!");
  } else {
    console.log("Queue authority already exists");
  }

  // Fund the queue authority PDA for task rewards
  const queueAuthorityBalance = await provider.connection.getBalance(queueAuthority);
  if (queueAuthorityBalance < 0.1 * LAMPORTS_PER_SOL) {
    console.log("Funding queue authority PDA...");
    await sendInstructions(provider, [
      SystemProgram.transfer({
        fromPubkey: wallet.publicKey,
        toPubkey: queueAuthority,
        lamports: 0.1 * LAMPORTS_PER_SOL,
      }),
    ]);
    console.log("Queue authority funded!");
  }

  console.log("\nSetup complete!");
  console.log("Task Queue:", taskQueue.toBase58());
  console.log("Queue Authority:", queueAuthority.toBase58());
  console.log("\nUpdate your test file with the task queue address.");
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });