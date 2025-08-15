#!/bin/bash
set -euo pipefail

# ===== Load environment =====
if [[ -n "${ENV_FILE:-}" && -f "${ENV_FILE}" ]]; then
  set -a; . "${ENV_FILE}"; set +a
elif [[ -f "./env.sh" ]]; then
  set -a; . ./env.sh; set +a
fi

: "${HOSTNAME:?HOSTNAME not set in env.sh}"
: "${EMAIL:?EMAIL not set in env.sh}"

export SYSTEMD_PAGER=""
export SYSTEMD_COLORS="0"

# ===== Sanity =====
if [[ "${HOSTNAME}" == "localhost" || "${HOSTNAME}" == "127.0.0.1" ]]; then
  echo "ERROR: HOSTNAME cannot be 'localhost' for Let's Encrypt. Set a public FQDN in env.sh." >&2
  exit 1
fi

echo -e "\n* * * Starting nginx configuration for landing page, reverse proxy and WSS * * *"

# ===== Ensure web root + landing page =====
WEBROOT="/var/www/${HOSTNAME}"
mkdir -p "${WEBROOT}"

LANDING_URL="https://raw.githubusercontent.com/nymtech/nym/refs/heads/feature/node-setup-cli/scripts/nym-node-setup/landing-page.html"
if curl -fsSL "${LANDING_URL}" -o "${WEBROOT}/index.html"; then
  echo "Landing page downloaded to ${WEBROOT}/index.html"
else
  echo "WARNING: Could not fetch landing page from ${LANDING_URL}. Creating placeholder."
  cat > "${WEBROOT}/index.html" <<'HTML'
<!doctype html><html><head><meta charset="utf-8"><title>Nym Node</title></head>
<body style="font-family:sans-serif;margin:2rem">
<h1>Nym node landing</h1>
<p>This is a placeholder page served by nginx.</p>
</body></html>
HTML
fi

# ===== Nginx base :80 site =====
SITES_AVAIL="/etc/nginx/sites-available"
SITES_EN="/etc/nginx/sites-enabled"
BASE_PATH="${SITES_AVAIL}/${HOSTNAME}"
BASE_LINK="${SITES_EN}/${HOSTNAME}"

[[ -L "${SITES_EN}/default" ]] && unlink "${SITES_EN}/default" || true

if [[ -f "${BASE_PATH}" ]]; then
  cp -f "${BASE_PATH}" "${BASE_PATH}.bak.$(date +%s)"
fi

cat > "${BASE_PATH}" <<EOF
server {
    listen 80;
    listen [::]:80;
    server_name ${HOSTNAME};

    root ${WEBROOT};
    index index.html;

    location = /favicon.ico { return 204; access_log off; log_not_found off; }

    location / {
        try_files \$uri \$uri/ @app;
    }

    location @app {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
    }
}
EOF

[[ -L "${BASE_LINK}" ]] && unlink "${BASE_LINK}" || true
ln -s "${BASE_PATH}" "${BASE_LINK}"

nginx -t
systemctl daemon-reexec
systemctl restart nginx

# ===== ACME preflight =====
echo -e "\n* * * ACME preflight checks * * *"
if ! curl -fsSL https://acme-v02.api.letsencrypt.org/directory >/dev/null; then
  echo "ERROR: Cannot reach Let's Encrypt directory." >&2
  exit 2
fi

THIS_IP="$(curl -fsS -4 https://ifconfig.me || true)"
DNS_IP="$(getent ahostsv4 "${HOSTNAME}" 2>/dev/null | awk '{print $1; exit}')"
echo "Public IPv4: ${THIS_IP:-unknown}   DNS A(${HOSTNAME}): ${DNS_IP:-unresolved}"
if [[ -n "${THIS_IP:-}" && -n "${DNS_IP:-}" && "${THIS_IP}" != "${DNS_IP}" ]]; then
  echo "WARNING: DNS for ${HOSTNAME} does not match this server's public IPv4."
fi

if ! timedatectl show -p NTPSynchronized --value 2>/dev/null | grep -qi yes; then
  timedatectl set-ntp true || true
fi

# ===== Certbot =====
if ! command -v certbot >/dev/null 2>&1; then
  if command -v snap >/dev/null 2>&1; then
    snap install core || true
    snap refresh core || true
    snap install --classic certbot
    ln -sf /snap/bin/certbot /usr/bin/certbot
  else
    apt update
    apt install -y certbot python3-certbot-nginx
  fi
fi

STAGING_FLAG=""
if [[ "${CERTBOT_STAGING:-0}" == "1" ]]; then
  STAGING_FLAG="--staging"
  echo "Using Let's Encrypt STAGING environment."
fi

certbot --nginx --non-interactive --agree-tos -m "${EMAIL}" -d "${HOSTNAME}" ${STAGING_FLAG} --redirect

LE_DIR="/etc/letsencrypt/live/${HOSTNAME}"
FULLCHAIN="${LE_DIR}/fullchain.pem"
PRIVKEY="${LE_DIR}/privkey.pem"

if [[ ! -s "${FULLCHAIN}" || ! -s "${PRIVKEY}" ]]; then
  echo "ERROR: Certificate files were not generated at ${LE_DIR}." >&2
  exit 3
fi

# ===== WSS :9001 site =====
WSS_AVAIL="/etc/nginx/sites-available/wss-config-nym"
WSS_LINK="/etc/nginx/sites-enabled/wss-config-nym"

if [[ -f "${WSS_AVAIL}" ]]; then
  cp -f "${WSS_AVAIL}" "${WSS_AVAIL}.bak.$(date +%s)"
fi

cat > "${WSS_AVAIL}" <<EOF
server {
    listen 9001 ssl http2;
    listen [::]:9001 ssl http2;

    server_name ${HOSTNAME};

    ssl_certificate ${FULLCHAIN};
    ssl_certificate_key ${PRIVKEY};
    include /etc/letsencrypt/options-ssl-nginx.conf;
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem;

    location = /favicon.ico { return 204; access_log off; log_not_found off; }

    location / {
        add_header 'Access-Control-Allow-Origin' '*';
        add_header 'Access-Control-Allow-Credentials' 'true';
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS, HEAD';
        add_header 'Access-Control-Allow-Headers' '*';

        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header X-Forwarded-For \$remote_addr;

        proxy_pass http://127.0.0.1:9000;
        proxy_intercept_errors on;
    }
}
EOF

[[ -L "${WSS_LINK}" ]] && unlink "${WSS_LINK}" || true
ln -s "${WSS_AVAIL}" "${WSS_LINK}"

nginx -t
systemctl daemon-reexec
systemctl restart nginx

echo -e "\nLanding page + SSL + WSS config completed for ${HOSTNAME}"
[[ "${STAGING_FLAG}" == "--staging" ]] && echo "NOTE: STAGING cert in use."
