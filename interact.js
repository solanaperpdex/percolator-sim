// interact-percolator.js
// Minimal interaction helper for Percolator (Router + Slab)
// - Checks presence of program accounts on your cluster
// - Derives PDAs per the repo README and tries to fetch them
// - Optionally simulates a no-op invoke to get program logs (no SOL spent)
//
// Usage (Windows PowerShell / VS Code terminal):
//   node .\interact-percolator.js --rpc https://api.devnet.solana.com --simulate
//   node .\interact-percolator.js --rpc http://127.0.0.1:8899 --market BTC-PERP --simulate
//   node .\interact-percolator.js --rpc http://127.0.0.1:8899 --router <ROUTER_PUBKEY> --slab <SLAB_PUBKEY> --simulate
//   node .\interact-percolator.js --help
//
// Flags:
//   --rpc <url>          RPC endpoint (default env RPC_URL or http://127.0.0.1:8899)
//   --simulate | -s      Simulate a no-op invoke to Router & Slab (no SOL spent)
//   --router <pubkey>    Override Router program ID (default vanity ID from README)
//   --slab <pubkey>      Override Slab program ID (default vanity ID from README)
//   --market <str>       Market string for slab PDA (default BTC-PERP)
//   --user <pubkey>      USER pubkey for PDAs (default payer)
//   --mint <pubkey>      MINT pubkey for PDAs (default USER)
//   --payer <path>       Path to keypair JSON (default ~/.config/solana/id.json or ephemeral)
//
// NOTE: This does NOT send real instructions to mutate state.

const fs = require('fs');
const os = require('os');
const path = require('path');
const {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
} = require('@solana/web3.js');

// ---------- CLI ARGS ----------
function printUsage() {
  console.log(`
Usage:
  node interact-percolator.js --rpc <url> [--simulate] [--router <pk>] [--slab <pk>] [--market <str>] [--user <pk>] [--mint <pk>] [--payer <file>]

Examples:
  node interact-percolator.js --rpc https://api.devnet.solana.com --simulate
  node interact-percolator.js --rpc http://127.0.0.1:8899 --router <ROUTER_PUBKEY> --slab <SLAB_PUBKEY> --simulate
`);
}

const argv = process.argv.slice(2);
const opts = {};
for (let i = 0; i < argv.length; i++) {
  const a = argv[i];
  if (a === '--help' || a === '-h') { printUsage(); process.exit(0); }
  else if (a === '--simulate' || a === '-s') opts.simulate = true;
  else if ((a === '--rpc' || a === '-r') && argv[i+1]) opts.rpc = argv[++i];
  else if (a === '--router' && argv[i+1]) opts.router = argv[++i];
  else if (a === '--slab'   && argv[i+1]) opts.slab   = argv[++i];
  else if (a === '--market' && argv[i+1]) opts.market = argv[++i];
  else if (a === '--user'   && argv[i+1]) opts.user   = argv[++i];
  else if (a === '--mint'   && argv[i+1]) opts.mint   = argv[++i];
  else if (a === '--payer'  && argv[i+1]) opts.payer  = argv[++i];
}

// ---------- Config ----------
const RPC_URL  = opts.rpc || process.env.RPC_URL || 'http://127.0.0.1:8899';
const MARKET   = opts.market || process.env.MARKET || 'BTC-PERP';
const SIMULATE = !!opts.simulate || process.env.SIMULATE === '1';

// Default vanity IDs from README (not deployed publicly by default)
const ROUTER_ID = new PublicKey(opts.router || process.env.ROUTER_ID || 'RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr');
const SLAB_ID   = new PublicKey(opts.slab   || process.env.SLAB_ID   || 'SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk');

// Known programs (sanity check)
const SYSTEM_ID = new PublicKey('11111111111111111111111111111111');
const MEMO_ID   = new PublicKey('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr');

// ---------- Helpers ----------
function tryLoadPayer() {
  // Priority: --payer path -> env PAYER path -> default Solana keypair -> ephemeral
  const candidate = opts.payer || process.env.PAYER || path.join(os.homedir(), '.config', 'solana', 'id.json');
  try {
    if (fs.existsSync(candidate)) {
      const raw = JSON.parse(fs.readFileSync(candidate, 'utf8'));
      return Keypair.fromSecretKey(Uint8Array.from(raw));
    }
  } catch (e) {
    console.warn('Failed to load payer from', candidate, e.message);
  }
  console.warn('No keypair file found; using ephemeral in-memory keypair.');
  return Keypair.generate();
}

function u64LE(nBigInt) {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(nBigInt));
  return b;
}

async function getPresence(connection, programId) {
  const info = await connection.getAccountInfo(programId);
  if (!info) return 'not found';
  return info.executable ? 'executable program' : 'non-executable account';
}

async function simulateNoop(connection, payer, programId) {
  const ix = new TransactionInstruction({
    keys: [{ pubkey: payer.publicKey, isSigner: true, isWritable: false }],
    programId,
    data: Buffer.alloc(0),
  });
  const tx = new Transaction().add(ix);
  tx.feePayer = payer.publicKey;
  tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
  tx.sign(payer);
  const res = await connection.simulateTransaction(tx, [payer]);
  return res.value;
}

(async () => {
  const connection = new Connection(RPC_URL, 'confirmed');
  const payer = tryLoadPayer();

  // USER/MINT defaults
  const USER = opts.user ? new PublicKey(opts.user)
             : (process.env.USER ? new PublicKey(process.env.USER) : payer.publicKey);
  const MINT = opts.mint ? new PublicKey(opts.mint)
             : (process.env.MINT ? new PublicKey(process.env.MINT) : USER);

  // RPC version (quick health-ish check)
  let version = {};
  try { version = await connection.getVersion(); } catch (_) {}

  console.log('RPC_URL      :', RPC_URL);
  console.log('RPC version  :', JSON.stringify(version));
  console.log('PAYER pubkey :', payer.publicKey.toBase58());
  console.log('USER  pubkey :', USER.toBase58());
  console.log('MINT  pubkey :', MINT.toBase58());
  console.log('MARKET       :', MARKET);
  console.log('SIMULATE     :', SIMULATE ? 'yes' : 'no');
  console.log('');

  // 1) Check program presence (include System/Memo sanity)
  console.log('Checking program presence...');
  const [systemPresence, memoPresence, routerPresence, slabPresence] = await Promise.all([
    getPresence(connection, SYSTEM_ID),
    getPresence(connection, MEMO_ID),
    getPresence(connection, ROUTER_ID),
    getPresence(connection, SLAB_ID),
  ]);
  console.log('System:', SYSTEM_ID.toBase58(), '→', systemPresence);
  console.log('Memo  :', MEMO_ID.toBase58(),   '→', memoPresence);
  console.log('Router:', ROUTER_ID.toBase58(), '→', routerPresence);
  console.log('Slab  :', SLAB_ID.toBase58(),   '→', slabPresence);
  console.log('');

  // 2) Derive PDAs (per README seeds)
  console.log('Deriving Router PDAs...');
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('vault'), MINT.toBuffer()],
    ROUTER_ID
  );
  const marketSeed = Buffer.from(MARKET); // slab "market_id" seed (ascii)
  const [slabStatePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('slab'), marketSeed],
    SLAB_ID
  );
  const [escrowPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('escrow'), USER.toBuffer(), slabStatePda.toBuffer(), MINT.toBuffer()],
    ROUTER_ID
  );
  const nonceU64 = u64LE(1n); // example nonce
  const [capPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('cap'), USER.toBuffer(), slabStatePda.toBuffer(), MINT.toBuffer(), nonceU64],
    ROUTER_ID
  );
  const [portfolioPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('portfolio'), USER.toBuffer()],
    ROUTER_ID
  );
  const [registryPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('registry')],
    ROUTER_ID
  );

  console.log('Vault     PDA:', vaultPda.toBase58());
  console.log('Escrow    PDA:', escrowPda.toBase58());
  console.log('Cap       PDA:', capPda.toBase58(), '(nonce=1)');
  console.log('Portfolio PDA:', portfolioPda.toBase58());
  console.log('Registry  PDA:', registryPda.toBase58());
  console.log('');

  console.log('Deriving Slab PDAs...');
  const [authorityPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('authority'), slabStatePda.toBuffer()],
    SLAB_ID
  );
  console.log('Slab State PDA:', slabStatePda.toBase58());
  console.log('Authority  PDA:', authorityPda.toBase58());
  console.log('');

  // 3) Try to fetch any of these accounts (exists/not)
  async function fetchMeta(pubkey) {
    const ai = await connection.getAccountInfo(pubkey);
    if (!ai) return { exists: false };
    return {
      exists: true,
      lamports: ai.lamports,
      executable: ai.executable,
      owner: ai.owner.toBase58(),
      dataLen: ai.data?.length ?? 0,
    };
  }

  console.log('Fetching account metas (exists / size / owner)...');
  const targets = [
    ['System', SYSTEM_ID],
    ['Memo', MEMO_ID],
    ['Router', ROUTER_ID],
    ['Slab', SLAB_ID],
    ['Vault PDA', vaultPda],
    ['Escrow PDA', escrowPda],
    ['Cap PDA', capPda],
    ['Portfolio PDA', portfolioPda],
    ['Registry PDA', registryPda],
    ['Slab State PDA', slabStatePda],
    ['Authority PDA', authorityPda],
  ];

  for (const [label, pk] of targets) {
    const meta = await fetchMeta(pk);
    if (!meta.exists) {
      console.log(`${label.padEnd(14)}: ${pk.toBase58()} → not found`);
    } else {
      console.log(
        `${label.padEnd(14)}: ${pk.toBase58()} → ` +
        `owner=${meta.owner} exec=${meta.executable} data=${meta.dataLen} lamports=${meta.lamports}`
      );
    }
  }
  console.log('');

  // 4) Optional: simulate a no-op invoke to each program (only if accounts exist)
  if (SIMULATE) {
    if (routerPresence === 'executable program') {
      console.log('Simulating no-op invoke to Router…');
      try {
        const res = await simulateNoop(connection, payer, ROUTER_ID);
        console.log('Router logs:', res.logs || []);
        console.log('Router err :', res.err || null);
      } catch (e) {
        console.error('Router simulate failed:', e.message);
      }
      console.log('');
    } else {
      console.log('Skipping Router simulate (account not found or not executable).');
    }

    if (slabPresence === 'executable program') {
      console.log('Simulating no-op invoke to Slab…');
      try {
        const res = await simulateNoop(connection, payer, SLAB_ID);
        console.log('Slab logs  :', res.logs || []);
        console.log('Slab err   :', res.err || null);
      } catch (e) {
        console.error('Slab simulate failed:', e.message);
      }
      console.log('');
    } else {
      console.log('Skipping Slab simulate (account not found or not executable).');
    }
  }

  console.log('Done.');
})().catch((e) => {
  console.error('Fatal:', e);
  process.exit(1);
});
