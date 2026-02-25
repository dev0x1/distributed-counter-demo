use anyhow::{Context as _, Result};
use clap::Parser;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tashi_vertex::{
    Context, Engine, KeyPublic, KeySecret, Message, Options, Peers, Socket, Transaction,
};
use tokio::sync::mpsc;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    node_id: String,

    #[arg(long)]
    bind: String,

    #[arg(long)]
    advertise: String,

    #[arg(long)]
    key_file: String,

    /// Peer entries like: BASE58PUB@host:port (repeatable)
    #[arg(long = "peer")]
    peers: Vec<String>,

    /// How often to propose an increment
    #[arg(long, default_value_t = 2)]
    inc_every_secs: u64,

    /// Increment amount
    #[arg(long, default_value_t = 1)]
    inc_amount: u64,
}

fn parse_peer(s: &str) -> Result<(KeyPublic, String)> {
    let (pubkey, addr) = s
        .split_once('@')
        .with_context(|| format!("peer must be PUB@ADDR, got: {s}"))?;
    let pk: KeyPublic = pubkey.parse().with_context(|| "parse KeyPublic")?;
    Ok((pk, addr.to_string()))
}

/// Transaction format:
///   INC <amount> <node_id> <nonce>
fn encode_inc(amount: u64, node_id: &str, nonce: u64) -> Vec<u8> {
    format!("INC {amount} {node_id} {nonce}").into_bytes()
}

fn try_parse_inc(tx: &[u8]) -> Option<u64> {
    // Remove trailing null byte if present (C-style string from engine)
    let bytes = if tx.last() == Some(&0u8) {
        &tx[..tx.len() - 1]
    } else {
        tx
    };

    // Convert to UTF-8
    let s = std::str::from_utf8(bytes).ok()?.trim();

    // Expected format:
    // INC <amount> <node_id> <nonce>
    let mut parts = s.split_whitespace();

    // Command must be "INC"
    if parts.next()? != "INC" {
        return None;
    }

    // Parse amount
    let amount = parts.next()?.parse::<u64>().ok()?;

    // Node ID (we don’t use it for state, but validate presence)
    let _node_id = parts.next()?;

    // Nonce (validate numeric)
    let _nonce = parts.next()?.parse::<u64>().ok()?;

    // No extra tokens allowed
    if parts.next().is_some() {
        return None;
    }

    Some(amount)
}

#[tokio::main] // multi-thread is fine; we keep Engine on this task
async fn main() -> Result<()> {
    let args = Args::parse();

    let secret_b58 = fs::read_to_string(&args.key_file)
        .with_context(|| format!("read key_file {}", args.key_file))?;
    let key: KeySecret = secret_b58
        .trim()
        .parse()
        .with_context(|| "parse KeySecret")?;

    // Peers
    let mut peers = Peers::new()?;
    // insert other peers from --peer
    for p in &args.peers {
        let (pk, addr) = parse_peer(p)?;
        peers.insert(&addr, &pk, Default::default())?;
    }

    // insert self using ADVERTISE (routable address), not bind
    peers.insert(&args.advertise, &key.public(), Default::default())?;

    println!(
        ":: {} bind={} advertise={}",
        args.node_id, args.bind, args.advertise
    );

    let context = Context::new()?;
    let socket = Socket::bind(&context, &args.bind).await?;
    let mut options = Options::default();
    options.set_report_gossip_events(true);
    options.set_fallen_behind_kick_s(10);

    let engine = Engine::start(&context, socket, options, &key, peers)?;

    println!(":: {} pubkey={}", args.node_id, key.public());

    // Local state
    let counter = AtomicU64::new(0);
    let nonce = AtomicU64::new(0);

    // Channel: ticker task -> main loop (Engine owner)
    let (tx_tick, mut rx_tick) = mpsc::unbounded_channel::<()>();

    // Spawn ticker WITHOUT capturing Engine (so it's Send-safe)
    {
        let interval_secs = args.inc_every_secs;
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                ticker.tick().await;
                // If receiver is gone, exit.
                if tx_tick.send(()).is_err() {
                    break;
                }
            }
        });
    }

    println!(
        ":: {} started distributed counter: inc_every={}s inc_amount={}",
        args.node_id, args.inc_every_secs, args.inc_amount
    );

    // Main loop owns Engine: handles both ticks (propose) and messages (apply)
    loop {
        tokio::select! {
            // propose an INC
            Some(_) = rx_tick.recv() => {
                let n = nonce.fetch_add(1, Ordering::Relaxed);
                let payload = encode_inc(args.inc_amount, &args.node_id, n);

                let mut t = Transaction::allocate(payload.len() + 1);
                t[..payload.len()].copy_from_slice(&payload);
                t[payload.len()] = 0;
                match engine.send_transaction(t) {
                    Ok(()) => println!(">> {} proposed INC {} (nonce={})", args.node_id, args.inc_amount, n),
                    Err(e) => eprintln!("!! {} send_transaction error: {:?}", args.node_id, e),
                }
            }

            // receive consensus output
            msg = engine.recv_message() => {
                let Some(message) = msg? else { break; };

                match message {
                    Message::SyncPoint(_) => {
                        println!("> {} got SYNCPOINT", args.node_id);
                    }
                    Message::Event(event) => {
                        let mut delta: u64 = 0;
                        let mut applied: u64 = 0;
                    println!(" > Received EVENT");

                    // Print event metadata
                    println!("    - From: {}", event.creator());
                    println!("    - Created: {}", event.created_at());
                    println!("    - Consensus: {}", event.consensus_at());
                    println!("    - Transactions: {}", event.transaction_count());
                   for tx in event.transactions() {
                            if let Some(amt) = try_parse_inc(tx.as_ref()) {
                                delta = delta.saturating_add(amt);
                                applied += 1;
                            }
                        }

                        if applied > 0 {
                            let new_val = counter.fetch_add(delta, Ordering::Relaxed).saturating_add(delta);
                            println!("> {} applied {} tx(s) (+{}), counter={}", args.node_id, applied, delta, new_val);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
