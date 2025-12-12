# LP (Lewes Protocol) Deployment Guide

## Prerequisites

### System Requirements

**Minimum:**
- CPU: 2 cores (x86_64 or ARM64)
- RAM: 4 GB
- Network: 100 Mbps
- Disk: 20 GB SSD

**Recommended:**
- CPU: 4+ cores with AVX2/NEON support (for SIMD optimizations)
- RAM: 8+ GB
- Network: 1 Gbps
- Disk: 50+ GB NVMe SSD

### Software Dependencies

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    postgresql \
    wireguard

# macOS
brew install \
    postgresql \
    wireguard-tools
```

## Gateway Setup

### 1. Enable LP in Configuration

Edit your gateway configuration file (typically `~/.nym/gateways/<id>/config/config.toml`):

```toml
[lp]
# Enable the LP listener
enabled = true

# Bind address (0.0.0.0 for all interfaces, 127.0.0.1 for localhost only)
bind_address = "0.0.0.0"

# Control port for LP handshake and registration
control_port = 41264

# Data port (reserved for future use, not currently used)
data_port = 51264

# Maximum concurrent LP connections
# Adjust based on expected load and available memory (~5 KB per connection)
max_connections = 10000

# Timestamp tolerance in seconds
# ClientHello messages with timestamps outside this window are rejected
# Balance security (smaller window) vs clock skew tolerance (larger window)
timestamp_tolerance_secs = 30

# IMPORTANT: ONLY for testing! Never enable in production
use_mock_ecash = false
```

### 2. Network Configuration

#### Firewall Rules

```bash
# Allow LP control port
sudo ufw allow 41264/tcp comment 'Nym LP control port'

# Optional: Rate limiting using iptables
sudo iptables -A INPUT -p tcp --dport 41264 -m state --state NEW \
    -m recent --set --name LP_CONN_LIMIT

sudo iptables -A INPUT -p tcp --dport 41264 -m state --state NEW \
    -m recent --update --seconds 60 --hitcount 100 --name LP_CONN_LIMIT \
    -j DROP
```

#### NAT/Port Forwarding

If your gateway is behind NAT, forward port 41264:

```bash
# Example for router at 192.168.1.1
# Forward external:41264 -> internal:41264 (TCP)

# Verify with:
nc -zv <your-public-ip> 41264
```

### 3. LP Keypair Generation

LP uses separate keypairs from the gateway's main identity. Generate on first run:

```bash
# Start gateway (will auto-generate LP keypair if missing)
./nym-node run --mode gateway --id <gateway-id>

# LP keypair stored at:
# ~/.nym/gateways/<id>/keys/lp_x25519.pem
```

**Key Storage Security:**

```bash
# Restrict key file permissions
chmod 600 ~/.nym/gateways/<id>/keys/lp_x25519.pem

# Backup keys securely (encrypted)
gpg -c ~/.nym/gateways/<id>/keys/lp_x25519.pem
# Store lp_x25519.pem.gpg in secure location
```

### 4. Database Configuration

LP requires PostgreSQL for credential tracking:

```bash
# Create database
sudo -u postgres createdb nym_gateway

# Create user
sudo -u postgres psql -c "CREATE USER nym_gateway WITH PASSWORD 'strong_password';"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE nym_gateway TO nym_gateway;"

# Configure in gateway config
[storage]
database_url = "postgresql://nym_gateway:strong_password@localhost/nym_gateway"
```

**Database Maintenance:**

```sql
-- Index for nullifier lookups (critical for performance)
CREATE INDEX idx_nullifiers ON spent_credentials(nullifier);

-- Periodic cleanup of old nullifiers (run daily via cron)
DELETE FROM spent_credentials WHERE expiry < NOW() - INTERVAL '30 days';

-- Vacuum to reclaim space
VACUUM ANALYZE spent_credentials;
```

### 5. WireGuard Configuration (for dVPN mode)

```bash
# Enable WireGuard kernel module
sudo modprobe wireguard

# Verify loaded
lsmod | grep wireguard

# Generate gateway WireGuard keys
wg genkey | tee wg_private.key | wg pubkey > wg_public.key
chmod 600 wg_private.key

# Configure in gateway config
[wireguard]
enabled = true
private_key_path = "/path/to/wg_private.key"
listen_port = 51820
interface_name = "wg-nym"
subnet = "10.0.0.0/8"
```

**WireGuard Interface Setup:**

```bash
# Create interface
sudo ip link add dev wg-nym type wireguard

# Configure interface
sudo ip addr add 10.0.0.1/8 dev wg-nym
sudo ip link set wg-nym up

# Enable IP forwarding
sudo sysctl -w net.ipv4.ip_forward=1
echo "net.ipv4.ip_forward=1" | sudo tee -a /etc/sysctl.conf

# NAT for WireGuard clients
sudo iptables -t nat -A POSTROUTING -s 10.0.0.0/8 -o eth0 -j MASQUERADE
```

### 6. Monitoring Setup

#### Prometheus Metrics

LP exposes metrics on the gateway's metrics endpoint (default: `:8080/metrics`):

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'nym-gateway-lp'
    static_configs:
      - targets: ['gateway-host:8080']
    metric_relabel_configs:
      # Focus on LP metrics
      - source_labels: [__name__]
        regex: 'lp_.*'
        action: keep
```

**Key Metrics:**

```promql
# Connection metrics
nym_gateway_active_lp_connections          # Current active connections
rate(nym_gateway_lp_connections_total[5m]) # Connection rate
rate(nym_gateway_lp_connections_completed_with_error[5m]) # Error rate

# Handshake metrics
rate(nym_gateway_lp_handshakes_success[5m])
rate(nym_gateway_lp_handshakes_failed[5m])
histogram_quantile(0.95, nym_gateway_lp_handshake_duration_seconds)

# Registration metrics
rate(nym_gateway_lp_registration_success_total[5m])
rate(nym_gateway_lp_registration_failed_total[5m])
histogram_quantile(0.95, nym_gateway_lp_registration_duration_seconds)

# Credential metrics
rate(nym_gateway_lp_credential_verification_failed[5m])
nym_gateway_lp_bandwidth_allocated_bytes_total

# Error metrics
rate(nym_gateway_lp_errors_handshake[5m])
rate(nym_gateway_lp_errors_timestamp_too_old[5m])
rate(nym_gateway_lp_errors_wg_peer_registration[5m])
```

#### Grafana Dashboard

Import dashboard JSON (create and export after setup):

```json
{
  "dashboard": {
    "title": "Nym Gateway - LP Protocol",
    "panels": [
      {
        "title": "Active Connections",
        "targets": [
          {
            "expr": "nym_gateway_active_lp_connections"
          }
        ]
      },
      {
        "title": "Registration Success Rate",
        "targets": [
          {
            "expr": "rate(nym_gateway_lp_registration_success_total[5m]) / (rate(nym_gateway_lp_registration_success_total[5m]) + rate(nym_gateway_lp_registration_failed_total[5m]))"
          }
        ]
      }
    ]
  }
}
```

#### Alert Rules

```yaml
# alerting_rules.yml
groups:
  - name: lp_alerts
    interval: 30s
    rules:
      # High connection rejection rate
      - alert: LPHighRejectionRate
        expr: rate(nym_gateway_lp_connections_completed_with_error[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High LP connection rejection rate"
          description: "Gateway {{ $labels.instance }} rejecting {{ $value }} connections/sec"

      # Handshake failure rate > 5%
      - alert: LPHandshakeFailures
        expr: |
          rate(nym_gateway_lp_handshakes_failed[5m]) /
          (rate(nym_gateway_lp_handshakes_success[5m]) + rate(nym_gateway_lp_handshakes_failed[5m]))
          > 0.05
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High LP handshake failure rate"

      # Credential verification issues
      - alert: LPCredentialVerificationFailures
        expr: rate(nym_gateway_lp_credential_verification_failed[5m]) > 50
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High credential verification failure rate"

      # High latency
      - alert: LPHighLatency
        expr: histogram_quantile(0.95, nym_gateway_lp_registration_duration_seconds) > 5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "LP registration latency is high"
```

## Client Configuration

### 1. Obtain Gateway LP Public Key

```bash
# Query gateway descriptor
curl https://validator.nymtech.net/api/v1/gateways/<gateway-identity>

# Extract LP public key from response
{
  "gateway": {
    "identity_key": "...",
    "lp_public_key": "base64-encoded-x25519-public-key",
    "host": "1.2.3.4",
    "lp_port": 41264
  }
}
```

### 2. Initialize Registration Client

```rust
use nym_registration_client::{RegistrationClient, RegistrationMode};

// Create client
let mut client = RegistrationClient::builder()
    .gateway_identity("gateway-identity-key")
    .gateway_lp_public_key(gateway_lp_pubkey)
    .gateway_lp_address("1.2.3.4:41264")
    .mode(RegistrationMode::Lp)
    .build()?;

// Perform registration
let result = client.register_lp(
    credential,          // E-cash credential
    RegistrationMode::Dvpn {
        wg_public_key: client_wg_pubkey,
    }
).await?;

match result {
    LpRegistrationResult::Success { gateway_data, .. } => {
        // Use gateway_data to configure WireGuard tunnel
    }
    LpRegistrationResult::Error { code, message } => {
        eprintln!("Registration failed: {}", message);
    }
}
```

## Testing

### Local Testing Environment

#### 1. Start Mock Gateway

```bash
# Use mock e-cash verifier (accepts any credential)
export LP_USE_MOCK_ECASH=true

# Start gateway in dev mode
./nym-node run --mode gateway --id test-gateway
```

#### 2. Test LP Connection

```bash
# Test TCP connectivity
nc -zv localhost 41264

# Test with openssl (basic TLS check - won't work as LP uses Noise)
timeout 5 openssl s_client -connect localhost:41264 < /dev/null
# Expected: Connection closes (Noise != TLS)
```

#### 3. Run Integration Tests

```bash
# Run full LP registration test suite
cargo test --test lp_integration -- --nocapture

# Run specific test
cargo test --test lp_integration test_dvpn_registration_success
```

### Production Testing

#### Health Check Script

```bash
#!/bin/bash
# lp_health_check.sh

GATEWAY_HOST="${1:-localhost}"
GATEWAY_PORT="${2:-41264}"

# Check TCP connectivity
if ! timeout 5 nc -zv "$GATEWAY_HOST" "$GATEWAY_PORT" 2>&1 | grep -q succeeded; then
    echo "CRITICAL: Cannot connect to LP port $GATEWAY_PORT"
    exit 2
fi

# Check metrics endpoint
ACTIVE_CONNS=$(curl -s "http://$GATEWAY_HOST:8080/metrics" | \
    grep "^nym_gateway_active_lp_connections" | awk '{print $2}')

if [ -z "$ACTIVE_CONNS" ]; then
    echo "WARNING: Cannot read metrics"
    exit 1
fi

echo "OK: LP listener responding, $ACTIVE_CONNS active connections"
exit 0
```

#### Load Testing

```bash
# Install tool
cargo install --git https://github.com/nymtech/nym tools/nym-lp-load-test

# Run load test (1000 concurrent registrations)
nym-lp-load-test \
    --gateway "1.2.3.4:41264" \
    --gateway-pubkey "base64-key" \
    --concurrent 1000 \
    --duration 60s
```

## Troubleshooting

### Connection Refused

**Symptom:** `Connection refused` when connecting to port 41264

**Diagnosis:**
```bash
# Check if LP listener is running
sudo netstat -tlnp | grep 41264

# Check gateway logs
journalctl -u nym-gateway -f | grep LP

# Check firewall
sudo ufw status | grep 41264
```

**Solutions:**
1. Ensure `lp.enabled = true` in config
2. Check bind address (`0.0.0.0` vs `127.0.0.1`)
3. Open firewall port: `sudo ufw allow 41264/tcp`
4. Restart gateway after config changes

### Handshake Failures

**Symptom:** `lp_handshakes_failed` metric increasing

**Diagnosis:**
```bash
# Check error logs
journalctl -u nym-gateway | grep "LP.*handshake.*failed"

# Common errors:
# - "Noise decryption error" → Wrong keys or MITM
# - "Timestamp too old" → Clock skew > 30s
# - "Replay detected" → Duplicate connection attempt
```

**Solutions:**
1. **Noise errors**: Verify client has correct gateway LP public key
2. **Timestamp errors**: Sync clocks with NTP
   ```bash
   sudo timedatectl set-ntp true
   sudo timedatectl status
   ```
3. **Replay errors**: Check for connection retry logic creating duplicates

### Credential Verification Failures

**Symptom:** `lp_credential_verification_failed` metric high

**Diagnosis:**
```bash
# Check database connectivity
psql -U nym_gateway -d nym_gateway -c "SELECT COUNT(*) FROM spent_credentials;"

# Check ecash manager logs
journalctl -u nym-gateway | grep -i credential
```

**Solutions:**
1. **Database errors**: Check PostgreSQL is running and accessible
2. **Signature errors**: Verify ecash contract address is correct
3. **Expired credentials**: Client needs to obtain fresh credentials
4. **Nullifier collision**: Credential already used (check `spent_credentials` table)

### High Latency

**Symptom:** `lp_registration_duration_seconds` p95 > 5 seconds

**Diagnosis:**
```bash
# Check database query performance
psql -U nym_gateway -d nym_gateway -c "EXPLAIN ANALYZE SELECT * FROM spent_credentials WHERE nullifier = 'test';"

# Check system load
top -bn1 | head -20
iostat -x 1 5
```

**Solutions:**
1. **Database slow**: Add index on nullifier column
   ```sql
   CREATE INDEX CONCURRENTLY idx_nullifiers ON spent_credentials(nullifier);
   ```
2. **CPU bound**: Check if SIMD is enabled
   ```bash
   # Check for AVX2 support
   grep avx2 /proc/cpuinfo
   # Rebuild with target-cpu=native
   RUSTFLAGS="-C target-cpu=native" cargo build --release
   ```
3. **Network latency**: Check RTT to gateway
   ```bash
   ping -c 10 gateway-host
   mtr gateway-host
   ```

### Connection Limit Reached

**Symptom:** `lp_connections_completed_with_error` high, logs show "connection limit exceeded"

**Diagnosis:**
```bash
# Check active connections
curl -s http://localhost:8080/metrics | grep active_lp_connections

# Check system limits
ulimit -n  # File descriptors per process
sysctl net.ipv4.ip_local_port_range
```

**Solutions:**
1. **Increase max_connections** in config:
   ```toml
   [lp]
   max_connections = 20000  # Increased from 10000
   ```
2. **Increase system limits**:
   ```bash
   # /etc/security/limits.conf
   nym-gateway soft nofile 65536
   nym-gateway hard nofile 65536

   # /etc/sysctl.conf
   net.ipv4.ip_local_port_range = 1024 65535
   net.core.somaxconn = 4096

   # Apply
   sudo sysctl -p
   ```
3. **Check for connection leaks**:
   ```bash
   # Connections in CLOSE_WAIT (indicates app not closing properly)
   netstat -an | grep 41264 | grep CLOSE_WAIT | wc -l
   ```

## Performance Tuning

### TCP Tuning

```bash
# /etc/sysctl.conf - Optimize for many concurrent connections

# Increase max backlog
net.core.somaxconn = 4096
net.ipv4.tcp_max_syn_backlog = 8192

# Faster TCP timeouts
net.ipv4.tcp_fin_timeout = 15
net.ipv4.tcp_keepalive_time = 300
net.ipv4.tcp_keepalive_probes = 5
net.ipv4.tcp_keepalive_intvl = 15

# Optimize buffer sizes
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.ipv4.tcp_rmem = 4096 87380 67108864
net.ipv4.tcp_wmem = 4096 65536 67108864

# Enable TCP Fast Open
net.ipv4.tcp_fastopen = 3

# Apply
sudo sysctl -p
```

### SIMD Optimization

Ensure gateway is built with CPU-specific optimizations:

```bash
# Check current CPU features
rustc --print target-features

# Build with native CPU features (enables AVX2, SSE4, etc.)
RUSTFLAGS="-C target-cpu=native" cargo build --release -p nym-node

# Verify SIMD is used (check binary for AVX2 instructions)
objdump -d target/release/nym-node | grep vpmovzxbw | wc -l
# Non-zero result means AVX2 is being used
```

### Database Optimization

```sql
-- Analyze query performance
EXPLAIN ANALYZE SELECT * FROM spent_credentials WHERE nullifier = 'xyz';

-- Essential indexes
CREATE INDEX CONCURRENTLY idx_spent_credentials_nullifier ON spent_credentials(nullifier);
CREATE INDEX CONCURRENTLY idx_spent_credentials_expiry ON spent_credentials(expiry);

-- Optimize PostgreSQL config (postgresql.conf)
-- Adjust based on available RAM
shared_buffers = 2GB                    # 25% of RAM
effective_cache_size = 6GB              # 75% of RAM
maintenance_work_mem = 512MB
work_mem = 64MB
max_connections = 200

-- Enable query planning optimizations
random_page_cost = 1.1                  # SSD-optimized
effective_io_concurrency = 200          # SSD-optimized

-- Restart PostgreSQL after config changes
sudo systemctl restart postgresql
```

## Security Hardening

### 1. Principle of Least Privilege

```bash
# Run gateway as dedicated user (not root)
sudo useradd -r -s /bin/false nym-gateway

# Set file ownership
sudo chown -R nym-gateway:nym-gateway /home/nym-gateway/.nym

# Systemd service with restrictions
[Service]
User=nym-gateway
Group=nym-gateway
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/home/nym-gateway/.nym
```

### 2. TLS for Metrics Endpoint

```bash
# Use reverse proxy (nginx) for metrics
server {
    listen 443 ssl http2;
    server_name metrics.your-gateway.com;

    ssl_certificate /etc/letsencrypt/live/metrics.your-gateway.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/metrics.your-gateway.com/privkey.pem;

    location /metrics {
        proxy_pass http://127.0.0.1:8080/metrics;
        # Authentication
        auth_basic "Metrics";
        auth_basic_user_file /etc/nginx/.htpasswd;
    }
}
```

### 3. Key Rotation

```bash
# Generate new LP keypair
./nym-node generate-lp-keypair --output new_lp_key.pem

# Atomic key swap (minimizes downtime)
# 1. Stop gateway gracefully
systemctl stop nym-gateway

# 2. Backup old key
cp ~/.nym/gateways/<id>/keys/lp_x25519.pem ~/.nym/gateways/<id>/keys/lp_x25519.pem.backup

# 3. Install new key
mv new_lp_key.pem ~/.nym/gateways/<id>/keys/lp_x25519.pem
chmod 600 ~/.nym/gateways/<id>/keys/lp_x25519.pem

# 4. Restart gateway
systemctl start nym-gateway

# 5. Update gateway descriptor (publishes new public key)
# This happens automatically on restart
```

## Maintenance

### Regular Tasks

**Daily:**
- Monitor metrics for anomalies
- Check error logs for new patterns
- Verify disk space for database growth

**Weekly:**
- Vacuum database to reclaim space
  ```sql
  VACUUM ANALYZE spent_credentials;
  ```
- Review and archive old logs
  ```bash
  journalctl --vacuum-time=7d
  ```

**Monthly:**
- Update dependencies (security patches)
  ```bash
  cargo update
  cargo audit
  cargo build --release
  ```
- Backup configuration and keys
- Review and update alert thresholds based on traffic patterns

**Quarterly:**
- Key rotation (if security policy requires)
- Performance review and capacity planning
- Security audit of configuration

### Backup Procedure

```bash
#!/bin/bash
# backup_lp.sh

BACKUP_DIR="/backup/nym-gateway/$(date +%Y%m%d)"
mkdir -p "$BACKUP_DIR"

# Backup keys
cp -r ~/.nym/gateways/<id>/keys "$BACKUP_DIR/"

# Backup config
cp ~/.nym/gateways/<id>/config/config.toml "$BACKUP_DIR/"

# Backup database
pg_dump -U nym_gateway nym_gateway | gzip > "$BACKUP_DIR/database.sql.gz"

# Encrypt and upload
tar -czf - "$BACKUP_DIR" | gpg -c | aws s3 cp - s3://backups/nym-gateway-$(date +%Y%m%d).tar.gz.gpg
```

### Upgrade Procedure

```bash
# 1. Backup current installation
./backup_lp.sh

# 2. Download new version
wget https://github.com/nymtech/nym/releases/download/vX.Y.Z/nym-node

# 3. Stop gateway
systemctl stop nym-gateway

# 4. Replace binary
sudo mv nym-node /usr/local/bin/nym-node
sudo chmod +x /usr/local/bin/nym-node

# 5. Run migrations (if any)
nym-node migrate --config ~/.nym/gateways/<id>/config/config.toml

# 6. Start gateway
systemctl start nym-gateway

# 7. Verify
curl http://localhost:8080/metrics | grep lp_connections_total
journalctl -u nym-gateway -f
```

## Reference

### Default Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 41264 | TCP | LP control plane (handshake + registration) |
| 51264 | Reserved | LP data plane (future use) |
| 51820 | UDP | WireGuard (for dVPN mode) |
| 8080 | HTTP | Metrics endpoint |

### File Locations

| File | Location | Purpose |
|------|----------|---------|
| Config | `~/.nym/gateways/<id>/config/config.toml` | Main configuration |
| LP Private Key | `~/.nym/gateways/<id>/keys/lp_x25519.pem` | LP static private key |
| WG Private Key | `~/.nym/gateways/<id>/keys/wg_private.key` | WireGuard private key |
| Database | PostgreSQL database | Nullifier tracking |
| Logs | `journalctl -u nym-gateway` | System logs |

### Useful Commands

```bash
# Check LP listener status
sudo netstat -tlnp | grep 41264

# View real-time logs
journalctl -u nym-gateway -f | grep LP

# Query metrics
curl -s http://localhost:8080/metrics | grep "^lp_"

# Check active connections
ss -tn sport = :41264 | wc -l

# Test credential verification
psql -U nym_gateway -d nym_gateway -c \
    "SELECT COUNT(*) FROM spent_credentials WHERE created_at > NOW() - INTERVAL '1 hour';"
```
