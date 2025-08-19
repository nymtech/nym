#!/usr/bin/env bash
set -euo pipefail

# ===== Load env (prefer absolute ENV_FILE injected by Python; fallback to ./env.sh) =====
if [[ -n "${ENV_FILE:-}" && -f "${ENV_FILE}" ]]; then
  set -a; . "${ENV_FILE}"; set +a
elif [[ -f "./env.sh" ]]; then
  set -a; . ./env.sh; set +a
fi

: "${HOSTNAME:?HOSTNAME not set in env.sh}"
: "${EMAIL:?EMAIL not set in env.sh}"

export SYSTEMD_PAGER=""
export SYSTEMD_COLORS="0"
DEBIAN_FRONTEND=noninteractive

# ===== Sanity =====
if [[ "${HOSTNAME}" == "localhost" || "${HOSTNAME}" == "127.0.0.1" ]]; then
  echo "ERROR: HOSTNAME cannot be 'localhost'. Use a public FQDN." >&2
  exit 1
fi

echo -e "\n* * * Starting nginx configuration for landing page, reverse proxy and WSS * * *"

# ===== Paths / Ports =====
WEBROOT="/var/www/${HOSTNAME}"
LE_ACME_DIR="/var/www/letsencrypt"
SITES_AVAIL="/etc/nginx/sites-available"
SITES_EN="/etc/nginx/sites-enabled"
BASE_HTTP="${SITES_AVAIL}/${HOSTNAME}"         # :80 vhost
BASE_HTTPS="${SITES_AVAIL}/${HOSTNAME}-ssl"    # :443 vhost (we’ll write it ourselves)
WSS_AVAIL="${SITES_AVAIL}/wss-config-nym"
BACKUP_DIR="/etc/nginx/sites-backups"

NYM_PORT_HTTP="${NYM_PORT_HTTP:-8080}"
NYM_PORT_WSS="${NYM_PORT_WSS:-9000}"
WSS_LISTEN_PORT="${WSS_LISTEN_PORT:-9001}"

mkdir -p "${WEBROOT}" "${LE_ACME_DIR}" "${BACKUP_DIR}" "${SITES_AVAIL}" "${SITES_EN}"

# ===== Helpers =====
neat_backup() {
  local file="$1"; [[ -f "$file" ]] || return 0
  local sha_now; sha_now="$(sha256sum "$file" | awk '{print $1}')" || return 0
  local tag; tag="$(basename "$file")"
  local latest="${BACKUP_DIR}/${tag}.latest"
  if [[ -f "$latest" ]]; then
    local sha_prev; sha_prev="$(awk '{print $1}' "$latest")"
    [[ "$sha_now" == "$sha_prev" ]] && return 0
  fi
  cp -a "$file" "${BACKUP_DIR}/${tag}.bak.$(date +%s)"
  echo "$sha_now  ${tag}" > "$latest"
  ls -1t "${BACKUP_DIR}/${tag}.bak."* 2>/dev/null | tail -n +6 | xargs -r rm -f
}

ensure_enabled() {
  local src="$1"; local name; name="$(basename "$src")"
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

reload_nginx() { nginx -t && systemctl reload nginx; }

# ===== Landing page (idempotent) =====
fetch_landing
echo "Landing page at ${WEBROOT}/index.html"

# ===== Disable default and stale SSL configs =====
[[ -L "${SITES_EN}/default" ]] && unlink "${SITES_EN}/default" || true
for f in "${SITES_EN}"/*; do
  [[ -L "$f" ]] || continue
  if grep -q "/etc/letsencrypt/live/localhost" "$f"; then
    echo "Disabling vhost referencing localhost cert: $f"; unlink "$f"
  fi
done
for f in "${SITES_EN}"/*; do
  [[ -L "$f" ]] || continue
  if grep -qE 'listen\s+.*443' "$f"; then
    cert=$(awk '/ssl_certificate[ \t]+/ {print $2}' "$f" | tr -d ';' | head -n1)
    key=$(awk '/ssl_certificate_key[ \t]+/ {print $2}' "$f" | tr -d ';' | head -n1)
    if [[ -n "${cert:-}" && ! -s "$cert" ]] || [[ -n "${key:-}" && ! -s "$key" ]]; then
      echo "Disabling SSL vhost with missing cert/key: $f"; unlink "$f"
    fi
  fi
done

# ===== HTTP :80 vhost (ACME-safe, proxy to :8080) =====
neat_backup "${BASE_HTTP}"
cat > "${BASE_HTTP}" <<EOF
server {
    listen 80;
    listen [::]:80;
    server_name ${HOSTNAME};

    # ACME challenge path (HTTP only)
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
ensure_enabled "${BASE_HTTP}"
reload_nginx
systemctl status nginx --no-pager | sed -n '1,6p' || true

# ===== ACME preflight (informative) =====
echo -e "\n* * * ACME preflight checks * * *"
if ! curl -fsSL https://acme-v02.api.letsencrypt.org/directory >/dev/null; then
  echo "WARNING: Can't reach Let's Encrypt directory. We'll still keep HTTP up." >&2
fi
THIS_IP="$(curl -fsS -4 https://ifconfig.me || true)"
DNS_IP="$(getent ahostsv4 "${HOSTNAME}" 2>/dev/null | awk '{print $1; exit}')"
echo "Public IPv4: ${THIS_IP:-unknown}   DNS A(${HOSTNAME}): ${DNS_IP:-unresolved}"
if [[ -n "${THIS_IP:-}" && -n "${DNS_IP:-}" && "${THIS_IP}" != "${DNS_IP}" ]]; then
  echo "WARNING: DNS for ${HOSTNAME} does not match this server's public IPv4."
fi
timedatectl show -p NTPSynchronized --value 2>/dev/null | grep -qi yes || timedatectl set-ntp true || true

# ===== Install certbot if missing =====
if ! command -v certbot >/dev/null 2>&1; then
  if command -v snap >/dev/null 2>&1; then
    snap install core || true; snap refresh core || true
    snap install --classic certbot; ln -sf /snap/bin/certbot /usr/bin/certbot
  else
    apt-get update -y >/dev/null 2>&1 || true
    apt-get install -y certbot >/dev/null 2>&1 || true
  fi
fi

# ===== Issue/renew via WEBROOT (no nginx auto-edit), non-fatal if it fails =====
STAGING_FLAG=""; [[ "${CERTBOT_STAGING:-0}" == "1" ]] && STAGING_FLAG="--staging" && echo "Using Let's Encrypt STAGING."
if ! cert_ok; then
  certbot certonly --non-interactive --agree-tos -m "${EMAIL}" -d "${HOSTNAME}" \
    --webroot -w "${LE_ACME_DIR}" ${STAGING_FLAG} || true
fi

# ===== Our own 443 vhost (only if certs exist) =====
if cert_ok; then
  neat_backup "${BASE_HTTPS}"
  cat > "${BASE_HTTPS}" <<EOF
server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name ${HOSTNAME};

    ssl_certificate /etc/letsencrypt/live/${HOSTNAME}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${HOSTNAME}/privkey.pem;
    include /etc/letsencrypt/options-ssl-nginx.conf;
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem;

    root ${WEBROOT};
    index index.html;

    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    location = /favicon.ico { return 204; access_log off; log_not_found off; }

    location / {
        try_files \$uri \$uri/ @app;
    }

    location @app {
        proxy_pass http://127.0.0.1:${NYM_PORT_HTTP};
        proxy_http_version 1.1;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
    }
}
EOF
  ensure_enabled "${BASE_HTTPS}"

  # Optional: redirect HTTP->HTTPS (keeps ACME path in HTTP too via separate small server)
  neat_backup "${BASE_HTTP}"
  cat > "${BASE_HTTP}" <<EOF
server {
    listen 80;
    listen [::]:80;
    server_name ${HOSTNAME};

    # Keep ACME reachable over HTTP:
    location ^~ /.well-known/acme-challenge/ {
        root ${LE_ACME_DIR};
        default_type "text/plain";
    }

    # Redirect the rest to HTTPS
    location / {
        return 301 https://\$host\$request_uri;
    }
}
EOF
  ensure_enabled "${BASE_HTTP}"
  reload_nginx
else
  echo "NOTE: Cert not present yet; HTTPS (443) will not listen. Only HTTP (80) is active."
fi

# ===== WSS TLS :9001 (only if certs exist) =====
if cert_ok; then
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
fi

echo -e "\nDone."
if cert_ok; then
  echo "HTTP : http://${HOSTNAME}/  (redirects to HTTPS)"
  echo "TLS  : https://${HOSTNAME}/  (served by nginx)"
  echo "WSS  : wss://${HOSTNAME}:${WSS_LISTEN_PORT}/  (served by nginx)"
else
  echo "Only HTTP is active (no cert yet). Re-run after DNS/ACME is ready to enable HTTPS + WSS."
fi
