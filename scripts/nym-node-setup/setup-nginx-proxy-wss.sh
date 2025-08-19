#!/usr/bin/env bash
set -euo pipefail

# load env (prefer absolute ENV_FILE injected by Python CLI; fallback to ./env.sh)
if [[ -n "${ENV_FILE:-}" && -f "${ENV_FILE}" ]]; then
  set -a; # export sourced vars
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
elif [[ -f "./env.sh" ]]; then
  set -a
  # shellcheck disable=SC1091
  . ./env.sh
  set +a
fi

: "${HOSTNAME:?HOSTNAME not set in env.sh}"
: "${EMAIL:?EMAIL not set in env.sh}"

export SYSTEMD_PAGER=""
export SYSTEMD_COLORS="0"
DEBIAN_FRONTEND=noninteractive

# sanity check
if [[ "${HOSTNAME}" == "localhost" || "${HOSTNAME}" == "127.0.0.1" ]]; then
  echo "ERROR: HOSTNAME cannot be 'localhost'. Use a public FQDN for Let's Encrypt." >&2
  exit 1
fi

echo -e "\n* * * Starting nginx configuration for landing page, reverse proxy and WSS * * *"

# define paths & ports
WEBROOT="/var/www/${HOSTNAME}"
LE_ACME_DIR="/var/www/letsencrypt"
SITES_AVAIL="/etc/nginx/sites-available"
SITES_EN="/etc/nginx/sites-enabled"
BASE_PATH="${SITES_AVAIL}/${HOSTNAME}"
BASE_LINK="${SITES_EN}/${HOSTNAME}"
WSS_AVAIL="${SITES_AVAIL}/wss-config-nym"
WSS_LINK="${SITES_EN}/wss-config-nym"
BACKUP_DIR="/etc/nginx/sites-backups"

NYM_PORT_HTTP="${NYM_PORT_HTTP:-8080}"      # nym-node HTTP (landing/proxy)
NYM_PORT_WSS="${NYM_PORT_WSS:-9000}"        # nym-node WSS upstream
WSS_LISTEN_PORT="${WSS_LISTEN_PORT:-9001}"  # public TLS WSS

mkdir -p "${WEBROOT}" "${LE_ACME_DIR}" "${BACKUP_DIR}"

# helpers
neat_backup() {
  local file="$1"
  [[ -f "$file" ]] || return 0
  local sha_now; sha_now="$(sha256sum "$file" | awk '{print $1}')" || return 0
  local tag; tag="$(basename "$file")"
  local latest="${BACKUP_DIR}/${tag}.latest"
  if [[ -f "$latest" ]]; then
    local sha_prev; sha_prev="$(awk '{print $1}' "$latest")"
    if [[ "$sha_now" == "$sha_prev" ]]; then
      return 0
    fi
  fi
  cp -a "$file" "${BACKUP_DIR}/${tag}.bak.$(date +%s)"
  echo "$sha_now  ${tag}" > "$latest"
  # keep last 5 backups
  ls -1t "${BACKUP_DIR}/${tag}.bak."* 2>/dev/null | tail -n +6 | xargs -r rm -f
}

ensure_enabled() {
  local src="$1"
  local name; name="$(basename "$src")"
  ln -sf "$src" "${SITES_EN}/${name}"
}

cert_ok() {
  [[ -s "/etc/letsencrypt/live/${HOSTNAME}/fullchain.pem" && -s "/etc/letsencrypt/live/${HOSTNAME}/privkey.pem" ]]
}

fetch_landing() {
  local url="https://raw.githubusercontent.com/nymtech/nym/refs/heads/feature/node-setup-cli/scripts/nym-node-setup/landing-page.html"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "${WEBROOT}/index.html" || true
  else
    wget -qO "${WEBROOT}/index.html" "$url" || true
  fi
  if [[ ! -s "${WEBROOT}/index.html" ]]; then
    cat > "${WEBROOT}/index.html" <<'HTML'
<!doctype html><html><head><meta charset="utf-8"><title>Nym Node</title></head>
<body style="font-family:sans-serif;margin:2rem">
<h1>Nym node landing</h1>
<p>This is a placeholder page served by nginx.</p>
</body></html>
HTML
  fi
}

reload_nginx() {
  nginx -t
  systemctl reload nginx
}

# landing page (idempotent)
fetch_landing
echo "Landing page at ${WEBROOT}/index.html"

# disable obvious stale/default blockers (safe)
[[ -L "${SITES_EN}/default" ]] && unlink "${SITES_EN}/default" || true

# disable any enabled site referencing localhost LE certs (avoids past crash you hit)
for f in "${SITES_EN}"/*; do
  [[ -L "$f" ]] || continue
  if grep -q "/etc/letsencrypt/live/localhost" "$f"; then
    echo "Disabling vhost referencing localhost cert: $f"
    unlink "$f"
  fi
done

# disable SSL vhosts with missing cert/key (don’t break nginx)
for f in "${SITES_EN}"/*; do
  [[ -L "$f" ]] || continue
  if grep -qE 'listen\s+.*443' "$f"; then
    cert=$(awk '/ssl_certificate[ \t]+/ {print $2}' "$f" | tr -d ';' | head -n1)
    key=$(awk '/ssl_certificate_key[ \t]+/ {print $2}' "$f" | tr -d ';' | head -n1)
    if [[ -n "${cert:-}" && ! -s "$cert" ]] || [[ -n "${key:-}" && ! -s "$key" ]]; then
      echo "Disabling SSL vhost with missing cert/key: $f"
      unlink "$f"
    fi
  fi
done

# plain HTTP site (port 80, ACME-safe, proxies to 8080)
neat_backup "${BASE_PATH}"
cat > "${BASE_PATH}" <<EOF
server {
    listen 80;
    listen [::]:80;
    server_name ${HOSTNAME};

    # ACME challenge path (must be HTTP reachable)
    location ^~ /.well-known/acme-challenge/ {
        root ${LE_ACME_DIR};
        default_type "text/plain";
    }

    root ${WEBROOT};
    index index.html;

    location = /favicon.ico { return 204; access_log off; log_not_found off; }

    location / {
        try_files \$uri \$uri/ @app;
    }

    location @app {
        proxy_pass http://127.0.0.1:${NYM_PORT_HTTP};
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
    }
}
EOF

ensure_enabled "${BASE_PATH}"
reload_nginx
systemctl status nginx --no-pager | sed -n '1,6p' || true

# ACME preflight
echo -e "\n* * * ACME preflight checks * * *"
if ! curl -fsSL https://acme-v02.api.letsencrypt.org/directory >/dev/null; then
  echo "ERROR: Cannot reach Let's Encrypt directory. Check outbound HTTPS / firewall / DNS." >&2
  # keep HTTP alive; skip cert for now
fi

THIS_IP="$(curl -fsS -4 https://ifconfig.me || true)"
DNS_IP="$(getent ahostsv4 "${HOSTNAME}" 2>/dev/null | awk '{print $1; exit}')"
echo "Public IPv4: ${THIS_IP:-unknown}   DNS A(${HOSTNAME}): ${DNS_IP:-unresolved}"
if [[ -n "${THIS_IP:-}" && -n "${DNS_IP:-}" && "${THIS_IP}" != "${DNS_IP}" ]]; then
  echo "WARNING: DNS for ${HOSTNAME} does not match this server's public IPv4. ACME challenge may fail."
fi

if ! timedatectl show -p NTPSynchronized --value 2>/dev/null | grep -qi yes; then
  echo "Enabling time sync (NTP)..."
  timedatectl set-ntp true || true
fi

# install certbot if missing
if ! command -v certbot >/dev/null 2>&1; then
  if command -v snap >/dev/null 2>&1; then
    echo -e "\n* * * Installing Certbot via snap * * *"
    snap install core || true
    snap refresh core || true
    snap install --classic certbot
    ln -sf /snap/bin/certbot /usr/bin/certbot
  else
    echo -e "\n* * * Installing Certbot via apt * * *"
    apt-get update -y >/dev/null 2>&1 || true
    apt-get install -y certbot python3-certbot-nginx >/dev/null 2>&1 || true
  fi
fi

# issue/renew certificate (non-fatal if it fails; we keep HTTP working)
STAGING_FLAG=""
if [[ "${CERTBOT_STAGING:-0}" == "1" ]]; then
  STAGING_FLAG="--staging"
  echo "Using Let's Encrypt STAGING."
fi

echo -e "\n* * * Requesting certificate for ${HOSTNAME} * * *"
if ! cert_ok; then
  certbot --nginx --non-interactive --agree-tos -m "${EMAIL}" -d "${HOSTNAME}" ${STAGING_FLAG} --redirect || true
  reload_nginx || true
fi

# WSS TLS site (only if certs exist)
if cert_ok; then
  echo -e "\n* * * Writing WSS TLS site on :${WSS_LISTEN_PORT} * * *"
  neat_backup "${WSS_AVAIL}"
  cat > "${WSS_AVAIL}" <<EOF
server {
    listen ${WSS_LISTEN_PORT} ssl http2;
    listen [::]:${WSS_LISTEN_PORT} ssl http2;

    server_name ${HOSTNAME};

    ssl_certificate /etc/letsencrypt/live/${HOSTNAME}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${HOSTNAME}/privkey.pem;
    include /etc/letsencrypt/options-ssl-nginx.conf;
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem;

    access_log /var/log/nginx/access.log;
    error_log  /var/log/nginx/error.log;

    location = /favicon.ico { return 204; access_log off; log_not_found off; }

    location / {
        add_header 'Access-Control-Allow-Origin' '*' always;
        add_header 'Access-Control-Allow-Credentials' 'true' always;
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS, HEAD' always;
        add_header 'Access-Control-Allow-Headers' '*' always;

        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header X-Forwarded-For \$remote_addr;

        proxy_pass http://127.0.0.1:${NYM_PORT_WSS};
        proxy_intercept_errors on;
    }
}
EOF

  ensure_enabled "${WSS_AVAIL}"
  reload_nginx
else
  echo "WARNING: Certificates for ${HOSTNAME} not found yet. Skipping WSS TLS site on :${WSS_LISTEN_PORT}."
  echo "         Re-run this script after DNS/ACME succeeds to enable WSS."
fi

echo -e "\nAll done."
echo "HTTP :80 for ${HOSTNAME} is active (with redirect if certbot enabled it)."
if cert_ok; then
  echo "WSS  :${WSS_LISTEN_PORT} TLS site is active using /etc/letsencrypt/live/${HOSTNAME}/ certs."
  [[ "${STAGING_FLAG}" == "--staging" ]] && echo "NOTE: STAGING cert in use. Re-run without CERTBOT_STAGING=1 for a real cert."
else
  echo "TLS cert not present yet — only HTTP is active. WSS will be enabled automatically on next run after cert exists."
fi

# quick checks
echo
echo "Local tests:"
echo "  curl -sv http://127.0.0.1/ -H 'Host: ${HOSTNAME}' | head -n 10"
echo "  ss -ltnp | egrep ':80|:${WSS_LISTEN_PORT}|:443' || true"
