# monkey-rust-vault

A secure, multi-provider file upload and storage service written in Rust. It exposes a unified REST API that routes file uploads to different cloud storage backends (AWS S3, Dropbox, Google Drive, or local filesystem) while encrypting credentials at rest and tracking all uploads in a Supabase PostgreSQL database.

## Features

- **Multi-provider storage** — S3, Dropbox, Google Drive, and local filesystem via a single endpoint
- **Credential encryption** — All provider secrets stored with AES-256-GCM encryption
- **Upload audit trail** — Every upload tracked in database with status (`pending` → `verified` | `failed`), SHA-256 checksum, and provider reference
- **Path deduplication** — Automatic collision resolution (`file.csv` → `file_1.csv`) when a path already exists
- **Upload verification** — Confirms file exists on the provider after upload before marking as verified
- **Instance/space tracking** — Optional `instance_id` and `space_id` fields for multi-tenant usage
- **Structured logging** — Built on the `tracing` crate with configurable log levels
- **Docker-ready** — Multi-stage Dockerfile producing a minimal Debian-based image

## Endpoints

### `POST /upload`

Upload a file to a configured storage bucket.

**Content-Type:** `multipart/form-data`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `uri` | text | Yes | Storage destination in the format `<scheme>://<bucket-slug>/<relative-path>` |
| `file` | binary | Yes | File data to upload |
| `instance_id` | UUID | No | Application instance identifier for tracking |
| `space_id` | UUID | No | Application space identifier for tracking |

**URI schemes:**

| Scheme | Provider |
|--------|----------|
| `s3://` | AWS S3 |
| `drop://` | Dropbox |
| `drive://` | Google Drive |
| `fs://` | Local filesystem |

**Example request:**
```bash
curl -X POST http://localhost:4200/upload \
  -F "uri=s3://my-bucket/reports/2024/report.csv" \
  -F "file=@/path/to/report.csv" \
  -F "instance_id=550e8400-e29b-41d4-a716-446655440000"
```

**Success response (200):**
```json
{
  "success": true,
  "uri": "s3://my-bucket/reports/2024/report.csv",
  "file_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Error responses:**

| Status | Cause |
|--------|-------|
| 400 | Invalid URI format or missing required fields |
| 404 | Bucket slug not found or provider is inactive |
| 500 | Upload failure, verification failure, or database error |

---

### `GET /health`

Health check endpoint. Returns `200 ok` when the service is running.

## Setup

### Prerequisites

- [Rust](https://rustup.rs/) (stable, edition 2024)
- A [Supabase](https://supabase.com/) project with service role access
- Storage provider credentials (AWS, Dropbox, Google Drive, or a local path)

### 1. Clone and configure environment

```bash
git clone <repo-url>
cd monkey-rust-vault
cp .env.example .env
```

Edit `.env` with your values:

```bash
# Supabase
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_SERVICE_KEY=your-service-role-key

# AES-256-GCM encryption key — must be exactly 64 hex characters (32 bytes)
# Generate one with: openssl rand -hex 32
VAULT_ENCRYPTION_KEY=your-64-char-hex-key

# AWS fallback region (used if the bucket row has no explicit region)
AWS_DEFAULT_REGION=ca-central-1

# Server bind address
BIND_ADDR=127.0.0.1:4200

# Log level
RUST_LOG=monkey_vault=debug,tower_http=debug
```

### 2. Run database migrations

Apply the SQL files in `migrations/` to your Supabase project in order:

```
migrations/001_providers.sql     — creates monkey_vault schema + storage_providers table
migrations/002_credentials.sql   — storage_provider_credentials table
migrations/003_buckets.sql       — storage_buckets table
migrations/004_files.sql         — storage_files table
```

You can run them via the Supabase SQL editor or any PostgreSQL client.

Then expose the schema to PostgREST in the **Supabase dashboard → API → Exposed schemas** — add `monkey_vault` to the list. Without this step all PostgREST requests to the schema will return 404.

### 3. Encrypt provider credentials

Use the included CLI tool to encrypt secrets before storing them in the database:

```bash
VAULT_ENCRYPTION_KEY=<your-key> cargo run --bin encrypt_secret -- "your-secret-value"
```

The output is the encrypted value to insert into `storage_provider_credentials`.

### 4. Seed database records

Insert rows in this order:

1. **`storage_providers`** — one row per provider type (`s3`, `dropbox`, `google_drive`, `local`)
2. **`storage_provider_credentials`** — encrypted credentials for each provider
3. **`storage_buckets`** — one row per logical bucket, referencing a provider and using a unique `slug`

### 5. Run the service

```bash
cargo run
```

The server starts on the address configured in `BIND_ADDR` (default `127.0.0.1:4200`).

## Building for production

### Release binary

```bash
cargo build --release
./target/release/monkey-vault
```

### Docker

```bash
docker build -t monkey-vault:latest .

docker run -p 4200:4200 \
  -e SUPABASE_URL="https://your-project.supabase.co" \
  -e SUPABASE_SERVICE_KEY="your-key" \
  -e VAULT_ENCRYPTION_KEY="your-64-char-hex-key" \
  -e AWS_DEFAULT_REGION="ca-central-1" \
  -e BIND_ADDR="0.0.0.0:4200" \
  monkey-vault:latest
```

### Musl target (for Linux deployments)

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

## Database schema

```
storage_providers (1)
    └── storage_provider_credentials (1)
    └── storage_buckets (many)
            └── storage_files (many)
```

| Table | Purpose |
|-------|---------|
| `storage_providers` | Defines available provider types and their active status |
| `storage_provider_credentials` | Encrypted credentials per provider |
| `storage_buckets` | Bucket configurations mapped to a slug used in URIs |
| `storage_files` | Upload history with status, checksum, and provider reference |

## Server Deployment (Ubuntu 24 + NGINX)

Applies to both **prod** (`vault.monkeylibrary.app`) and **staging** (`vx_vault.venux-channel.com`). Each runs on its own server; repeat these steps on each.

### 1. Install dependencies

```bash
sudo apt update && sudo apt install -y nginx certbot python3-certbot-nginx
```

### 2. Create a dedicated system user

```bash
sudo useradd -r -s /bin/false monkey-vault
```

### 3. Create directories

```bash
sudo mkdir -p /opt/monkey-vault
sudo mkdir -p /etc/monkey-vault
sudo chown monkey-vault:monkey-vault /opt/monkey-vault
```

### 4. Install Rust on the server

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Verify:

```bash
rustc --version
```

> First `cargo build --release` takes 2–3 min. Subsequent deploys are fast thanks to incremental compilation.

### 5. Allow the deploy user to restart the service

Add a narrow sudoers rule so Deployer can restart the systemd service without a password prompt:

```bash
sudo visudo -f /etc/sudoers.d/monkey-vault
```

Add this line:

```
fabricio ALL=(ALL) NOPASSWD: /bin/systemctl restart monkey-vault
```

### 6. Install the systemd service

```bash
sudo cp server/monkey-vault.service /etc/systemd/system/monkey-vault.service
sudo systemctl daemon-reload
sudo systemctl enable monkey-vault
```

> The service points to Deployer's `current/` symlink — it will auto-pick up every new release on restart.

### 7. Create the shared `.env` on the server

Deployer keeps `.env` outside the release directory so it persists across deploys. Create it once:

```bash
mkdir -p /home/fabricio/Apps/monkey-vault/shared
nano /home/fabricio/Apps/monkey-vault/shared/.env
```

Paste and fill in your values:

```bash
BIND_ADDR=127.0.0.1:4200

SUPABASE_URL=https://your-project.supabase.co
SUPABASE_SERVICE_KEY=your-service-role-key

VAULT_ENCRYPTION_KEY=your-64-char-hex-key   # openssl rand -hex 32

AWS_DEFAULT_REGION=us-east-1

RUST_LOG=monkey_vault=info,tower_http=info
```

### 8. Configure NGINX

```bash
# Prod
sudo cp server/nginx_prod.conf /etc/nginx/sites-available/monkey-vault
# Staging
sudo cp server/nginx_staging.conf /etc/nginx/sites-available/monkey-vault

sudo ln -s /etc/nginx/sites-available/monkey-vault /etc/nginx/sites-enabled/monkey-vault
sudo nginx -t && sudo systemctl reload nginx
```

### 9. Issue the SSL certificate

Make sure the domain DNS A record points to this server first.

```bash
# Prod
sudo certbot --nginx -d vault.monkeylibrary.app
# Staging
sudo certbot --nginx -d vx_vault.venux-channel.com
```

Certbot updates the NGINX config automatically. Verify auto-renewal:

```bash
sudo systemctl status certbot.timer
```

### 10. Deploy with Deployer

Install Deployer locally if you haven't:

```bash
composer require --dev deployer/deployer
```

First deploy (seeds the release structure):

```bash
# Prod
vendor/bin/dep deploy prod

# Staging
vendor/bin/dep deploy staging
```

After the first deploy the service starts automatically. Verify:

```bash
curl https://vault.monkeylibrary.app/health
# → ok
```

### Deploying updates

```bash
# Prod
vendor/bin/dep deploy prod

# Staging
vendor/bin/dep deploy staging
```

Deployer will: pull the latest code → build → atomically symlink → restart the service.

### Rollback

```bash
vendor/bin/dep rollback prod
```

Deployer re-symlinks to the previous release and restarts the service. Up to 3 releases are kept (`keep_releases = 3` in `deploy.php`).

### Useful commands

```bash
# View live logs
sudo journalctl -u monkey-vault -f

# Restart service manually
sudo systemctl restart monkey-vault

# Check NGINX config
sudo nginx -t

# Renew SSL manually
sudo certbot renew --dry-run
```

## Development

```bash
# Type check
cargo check

# Lint (zero warnings enforced)
cargo clippy --all-targets -- -D warnings

# Run tests
cargo test
```

## Project structure

```
src/
├── main.rs               — Entry point, router setup
├── config.rs             — Environment variable loading
├── error.rs              — Error types and HTTP mapping
├── models.rs             — Shared data structures
├── crypto.rs             — AES-256-GCM encrypt/decrypt
├── service.rs            — Core upload logic and path deduplication
├── supabase.rs           — Supabase PostgREST client
├── uri.rs                — URI scheme parser
├── db/mod.rs             — Database queries
├── routes/upload.rs      — POST /upload handler
└── providers/
    ├── mod.rs            — StorageDriver trait and factory
    ├── s3.rs             — AWS S3 implementation
    ├── dropbox.rs        — Dropbox API implementation
    ├── google_drive.rs   — Google Drive implementation
    └── local.rs          — Local filesystem implementation
```
