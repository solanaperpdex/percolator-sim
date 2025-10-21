// dashboard.js
// Visual dashboard for your local validator + Percolator PDAs
// Runs a small web server at http://localhost:3000

const fs = require('fs');
const os = require('os');
const path = require('path');
const express = require('express');
const {
  Connection,
  Keypair,
  PublicKey,
} = require('@solana/web3.js');

const PORT = process.env.PORT || 3000;
const DEFAULT_RPC = process.env.RPC_URL || 'http://127.0.0.1:8899';
const DEFAULT_MARKET = 'BTC-PERP';

// Vanity IDs from README (may not be deployed)
const DEFAULT_ROUTER = 'RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr';
const DEFAULT_SLAB   = 'SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk';

// Known programs (for sanity)
const SYSTEM_ID = '11111111111111111111111111111111';
const MEMO_ID   = 'MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr';

function tryLoadPayer() {
  const candidate = process.env.PAYER || path.join(os.homedir(), '.config', 'solana', 'id.json');
  try {
    if (fs.existsSync(candidate)) {
      const raw = JSON.parse(fs.readFileSync(candidate, 'utf8'));
      return Keypair.fromSecretKey(Uint8Array.from(raw));
    }
  } catch (e) {
    console.warn('Failed to load payer from', candidate, e.message);
  }
  console.warn('No keypair file found; using ephemeral keypair.');
  return Keypair.generate();
}

function u64LE(nBigInt) {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(nBigInt));
  return b;
}

async function getPresence(connection, pk) {
  const info = await connection.getAccountInfo(new PublicKey(pk));
  if (!info) return { status: 'not found' };
  return {
    status: info.executable ? 'executable program' : 'non-executable account',
    lamports: info.lamports,
    owner: info.owner.toBase58(),
    dataLen: info.data?.length ?? 0,
  };
}

function deriveAllPDAs({ routerPk, slabPk, userPk, mintPk, market }) {
  const ROUTER_ID = new PublicKey(routerPk);
  const SLAB_ID   = new PublicKey(slabPk);
  const USER      = new PublicKey(userPk);
  const MINT      = new PublicKey(mintPk);

  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('vault'), MINT.toBuffer()],
    ROUTER_ID
  );
  const marketSeed = Buffer.from(market);
  const [slabStatePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('slab'), marketSeed],
    SLAB_ID
  );
  const [escrowPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('escrow'), USER.toBuffer(), slabStatePda.toBuffer(), MINT.toBuffer()],
    ROUTER_ID
  );
  const nonceU64 = u64LE(1n);
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
  const [authorityPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('authority'), slabStatePda.toBuffer()],
    SLAB_ID
  );

  return {
    vaultPda: vaultPda.toBase58(),
    escrowPda: escrowPda.toBase58(),
    capPda: capPda.toBase58(),
    portfolioPda: portfolioPda.toBase58(),
    registryPda: registryPda.toBase58(),
    slabStatePda: slabStatePda.toBase58(),
    authorityPda: authorityPda.toBase58(),
  };
}

async function fetchMeta(connection, base58) {
  const ai = await connection.getAccountInfo(new PublicKey(base58));
  if (!ai) return { exists: false };
  return {
    exists: true,
    base58,
    lamports: ai.lamports,
    executable: ai.executable,
    owner: ai.owner.toBase58(),
    dataLen: ai.data?.length ?? 0,
  };
}

const app = express();

app.get('/', (_req, res) => {
  res.send(`
<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <title>Percolator Local Dashboard</title>
  <style>
    body { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; background:#0b0b12; color:#e5e7eb; margin:0; }
    header { padding:16px 24px; background:#111118; border-bottom:1px solid #1f2330; display:flex; justify-content:space-between; align-items:center; }
    h1 { margin:0; font-size:18px; color:#a78bfa; }
    .controls { display:flex; gap:8px; align-items:center; }
    input, button { background:#0f1320; color:#e5e7eb; border:1px solid #252a3a; padding:8px 10px; border-radius:8px; }
    button { cursor:pointer; }
    main { padding:24px; max-width:1100px; margin:0 auto; }
    .grid { display:grid; grid-template-columns: repeat(2, minmax(0,1fr)); gap:16px; }
    .card { background:#0f1320; border:1px solid #252a3a; border-radius:12px; padding:16px; }
    .title { font-weight:600; color:#93c5fd; margin-bottom:6px; }
    .mono { font-family: inherit; opacity:0.9; word-break: break-all; }
    table { width:100%; border-collapse:collapse; margin-top:8px; }
    th, td { text-align:left; padding:6px 8px; border-bottom:1px solid #1b2030; font-size:13px; }
    .pill { display:inline-block; padding:2px 8px; border-radius:12px; font-size:12px; }
    .ok { background:#052e16; color:#86efac; border:1px solid #14532d; }
    .warn { background:#3f1d1d; color:#fca5a5; border:1px solid #7f1d1d; }
    .muted { color:#9aa3b2; }
    .row { display:flex; gap:16px; flex-wrap:wrap; }
    .small { font-size:12px; }
  </style>
</head>
<body>
  <header>
    <h1>Percolator • Local Dashboard</h1>
    <div class="controls">
      <input id="rpc" size="34" value="http://127.0.0.1:8899" />
      <input id="router" size="44" value="${DEFAULT_ROUTER}" />
      <input id="slab" size="44" value="${DEFAULT_SLAB}" />
      <input id="market" size="12" value="${DEFAULT_MARKET}" />
      <button onclick="refresh()">Refresh</button>
      <button onclick="autoToggle()" id="autoBtn">Auto: Off</button>
    </div>
  </header>
  <main>
    <div id="summary" class="row"></div>
    <div class="grid" style="margin-top:16px">
      <div class="card">
        <div class="title">Program Presence</div>
        <table id="presence"><tbody></tbody></table>
      </div>
      <div class="card">
        <div class="title">Derived PDAs</div>
        <table id="pdas"><tbody></tbody></table>
      </div>
      <div class="card" style="grid-column: span 2;">
        <div class="title">Account Metadata</div>
        <table id="metas"><thead><tr><th>Label</th><th>Pubkey</th><th>Owner</th><th>Exec</th><th>Data</th><th>Lamports</th></tr></thead><tbody></tbody></table>
      </div>
    </div>
  </main>

<script>
let auto = false, timer = null;

function pill(text, ok) {
  const cls = 'pill ' + (ok ? 'ok' : 'warn');
  return '<span class="'+cls+'">'+text+'</span>';
}

async function refresh() {
  const rpc = document.getElementById('rpc').value.trim();
  const router = document.getElementById('router').value.trim();
  const slab = document.getElementById('slab').value.trim();
  const market = document.getElementById('market').value.trim();
  const res = await fetch(\`/api/status?rpc=\${encodeURIComponent(rpc)}&router=\${router}&slab=\${slab}&market=\${market}\`);
  const data = await res.json();

  // Summary
  const sum = document.getElementById('summary');
  sum.innerHTML = \`
    <div class="card" style="flex:1">
      <div class="small muted">RPC</div>
      <div class="mono">\${data.rpc.url}</div>
      <div class="small muted">Version</div>
      <div class="mono">\${JSON.stringify(data.rpc.version)}</div>
      <div class="small muted">Payer</div>
      <div class="mono">\${data.payer}</div>
      <div class="small muted">USER / MINT</div>
      <div class="mono">\${data.user} / \${data.mint}</div>
    </div>
  \`;

  // Presence
  const pres = document.querySelector('#presence tbody');
  pres.innerHTML = '';
  for (const row of data.presence) {
    const ok = row.status.includes('executable');
    pres.innerHTML += \`
      <tr>
        <td class="mono">\${row.label}</td>
        <td class="mono">\${row.pubkey}</td>
        <td>\${ok ? pill(row.status, true) : pill(row.status, false)}</td>
      </tr>\`;
  }

  // PDAs
  const pdas = document.querySelector('#pdas tbody');
  pdas.innerHTML = '';
  for (const [label, pk] of data.pdas) {
    pdas.innerHTML += \`<tr><td class="mono">\${label}</td><td class="mono">\${pk}</td></tr>\`;
  }

  // Metas
  const metas = document.querySelector('#metas tbody');
  metas.innerHTML = '';
  for (const m of data.metas) {
    metas.innerHTML += \`
      <tr>
        <td>\${m.label}</td>
        <td class="mono">\${m.pubkey}</td>
        <td class="mono">\${m.owner || '-'}</td>
        <td>\${m.exec ?? '-'}</td>
        <td>\${m.data ?? '-'}</td>
        <td>\${m.lamports ?? '-'}</td>
      </tr>\`;
  }
}

function autoToggle() {
  auto = !auto;
  document.getElementById('autoBtn').textContent = 'Auto: ' + (auto ? 'On' : 'Off');
  if (auto) {
    refresh();
    timer = setInterval(refresh, 2000);
  } else {
    clearInterval(timer);
  }
}

refresh();
</script>
</body>
</html>
  `);
});

app.get('/api/status', async (req, res) => {
  const rpc = (req.query.rpc || DEFAULT_RPC).toString();
  const routerPk = (req.query.router || DEFAULT_ROUTER).toString();
  const slabPk   = (req.query.slab   || DEFAULT_SLAB).toString();
  const market   = (req.query.market || DEFAULT_MARKET).toString();

  try {
    const connection = new Connection(rpc, 'confirmed');
    let version = {};
    try { version = await connection.getVersion(); } catch (_) {}

    const payer = tryLoadPayer();
    const userPk = payer.publicKey.toBase58();
    const mintPk = userPk;

    // presence
    const pres = await Promise.all([
      (async () => ({ label: 'System', pubkey: SYSTEM_ID, ...(await getPresence(connection, SYSTEM_ID)) }))(),
      (async () => ({ label: 'Memo',   pubkey: MEMO_ID,   ...(await getPresence(connection, MEMO_ID)) }))(),
      (async () => ({ label: 'Router', pubkey: routerPk,  ...(await getPresence(connection, routerPk)) }))(),
      (async () => ({ label: 'Slab',   pubkey: slabPk,    ...(await getPresence(connection, slabPk)) }))(),
    ]);

    // PDAs
    const p = deriveAllPDAs({ routerPk, slabPk, userPk, mintPk, market });
    const pdas = [
      ['Vault PDA', p.vaultPda],
      ['Escrow PDA', p.escrowPda],
      ['Cap PDA (nonce=1)', p.capPda],
      ['Portfolio PDA', p.portfolioPda],
      ['Registry PDA', p.registryPda],
      ['Slab State PDA', p.slabStatePda],
      ['Authority PDA', p.authorityPda],
    ];

    // metas
    const labels = [
      ['System', SYSTEM_ID],
      ['Memo', MEMO_ID],
      ['Router', routerPk],
      ['Slab', slabPk],
      ...pdas,
    ];
    const metas = [];
    for (const [label, pk] of labels) {
      const ai = await connection.getAccountInfo(new PublicKey(pk));
      metas.push({
        label,
        pubkey: pk,
        owner: ai ? ai.owner.toBase58() : null,
        exec: ai ? ai.executable : null,
        data: ai ? (ai.data?.length ?? 0) : null,
        lamports: ai ? ai.lamports : null,
      });
    }

    res.json({
      rpc: { url: rpc, version },
      payer: userPk,
      user: userPk,
      mint: mintPk,
      presence: pres,
      pdas,
      metas,
    });
  } catch (e) {
    res.status(500).json({ error: e.message || String(e) });
  }
});

app.listen(PORT, () => {
  console.log(`Percolator dashboard: http://localhost:${PORT}`);
  console.log(`Default RPC: ${DEFAULT_RPC}`);
});
