# Distributed Counter Demo Using Tashi Vertex

This demo runs a **4-node cluster** using [`tashi-vertex-rs`](https://github.com/tashigg/tashi-vertex-rs) and a small `/demo` Rust binary that implements a **distributed counter**:

- Each node periodically proposes an increment transaction: `INC <amount> <node_id> <nonce>`
- Nodes apply increments **only when they appear in consensus events**, so state converges.

The setup is designed to be reproducible on macOS (Docker Desktop) and runs the containers as **linux/amd64**.

---

## What’s in this repo

```

.
├── Dockerfile
├── docker-compose.yml
├── demo/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # distributed counter node
│       └── bin/keygen.rs   # generates per-node keypairs into /keys (volume)
└── scripts/
├── node-entrypoint.sh  # resolves peers + starts node with bind/advertise
└── wait-for-keys.sh    # waits for key volume to be populated

````

---

## How it works (high level)

### 1) Key generation
A `keygen` service generates **4 keypairs** into a Docker volume mounted at `/keys`:

- `/keys/node1.secret`, `/keys/node1.public`
- ...
- `/keys/node4.secret`, `/keys/node4.public`

This runs once per volume (idempotent). To regenerate keys, remove volumes (see below).

### 2) Node startup (4 nodes)
Each node container:

- Waits for keys to exist in `/keys`
- Builds the peer list as `PUBKEY@IP:PORT`
- Uses:
  - `--bind 0.0.0.0:800X` (listen on all interfaces)
  - `--advertise <container-ip>:800X` (routable endpoint other nodes can dial)

### 3) Replicated state machine
- Every `inc_every_secs`, a node proposes a transaction: `INC 1 nodeX nonce`
- When the engine emits a consensus **Event**, the node parses transactions and updates its local `counter`.
- Nodes print logs on propose + apply.

---

## Prerequisites

- Docker Desktop
- docker compose (v2)

---

## Build and Run

From the repo root:

```bash
docker compose up --build
````

You should see logs like:

* `>> node2 proposed INC 1 (nonce=0)`
* `> node1 applied 3 tx(s) (+3), counter=42`
* `> node3 got SYNCPOINT`

> Note: depending on configuration, you may see SYNCPOINTs early before events start flowing.

---

## Ports

The demo publishes node ports to the host (for visibility / debugging):

* node1: host `18001` → container `8001`
* node2: host `18002` → container `8002`
* node3: host `18003` → container `8003`
* node4: host `18004` → container `8004`

If you don’t need host publishing, you can remove the `ports:` mappings and keep everything internal to the compose network.

---

## Reset / clean slate

### Stop services

```bash
docker compose down
```

### Remove keys and regenerate them (recommended when debugging membership / keys)

```bash
docker compose down -v
```

This deletes the `keys` volume so `keygen` will generate fresh keypairs next run.

### Full rebuild (avoid cache issues across platform changes)

```bash
docker compose down -v
docker compose build --no-cache --pull
docker compose up
```

---

## Helpful commands

Tail logs:

```bash
docker compose logs -f
```

Inspect containers:

```bash
docker compose ps
```

Exec into a node:

```bash
docker exec -it dist-counter-node1-1 bash
```

Check keys volume:

```bash
docker exec -it dist-counter-node1-1 bash -lc "ls -l /keys && head -n 1 /keys/node1.public"
```

---