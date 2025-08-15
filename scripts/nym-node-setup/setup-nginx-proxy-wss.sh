#!/bin/bash
set -euo pipefail

# --- Load env.sh (absolute path via ENV_FILE if provided), else try ./env.sh ---
if [[ -n "${ENV_FILE:-}" && -f "${ENV_FILE}" ]]; then
  set -a; . "${ENV_FILE}"; set +a
elif [[ -f "./env.sh" ]]; then
  set -a; . ./env.sh; set +a
fi

: "${HOSTNAME:?HOSTNAME not set. Put it in env.sh}"
: "${EMAIL:?EMAIL not set. Put it in env.sh}"

echo -e "\n* * * Preflight checks for ACME * * *"
# 1) System time sanity (ACME rejects skewed clocks)
if ! curl -fsSL https://acme-v02.api.letsencrypt.org/directory >/dev/null; then
  echo "ERROR: Cannot reach Let's Encrypt. Check outbound HTTPS/Firewall/DNS." >&2
  exit 1
fi
if ! timedatectl show -p NTPSynchronized --value 2>/dev/null | grep -qi yes; then
  echo "WARNING: NTP not synchronized. Enabling..."
  timedatectl set-ntp true || true
fi

# 2) DNS sanity (hostname must resolve to this box)
THIS_IP="$(curl -fsS -4 https://ifconfig.me || true)"
DNS_IP="$(getent ahostsv4 "$HOSTNAME" 2>/dev/null | awk '{print $1; exit}')"
echo "Public IPv4: ${THIS_IP:-unknown}  DNS A(${HOSTNAME}): ${DNS_IP:-unresolved}"
if [[ -z "${DNS_IP:-}" ]]; then
  echo "ERROR: ${HOSTNAME} does not resolve to an IPv4 address." >&2
  exit 1
fi

# 3) Nginx base landing + reverse proxy on :80
LANDING_PAGE_PATH="/etc/nginx/sites-available/${HOSTNAME}"
LANDING_PAGE_LINK="/etc/nginx/sites-enabled/${HOSTNAME}"
echo -e "\n* * * Starting nginx configuration for landing page, reverse proxy and WSS * * *"
mkdir -p "/var/www/${HOSTNAME}"
cp ./landing-page.html "/var/www/${HOSTNAME}/index.html" || true

[ -L /etc/nginx/sites-enabled/default ] && unlink /etc/nginx/sites-enabled/default || true
[ -L "$LANDING_PAGE_LINK" ] && unlink "$LANDING_PAGE_LINK" || true

cat > "$LANDING_PAGE_PATH" <<EOF
server {
    listen 80;
    listen [::]:80;
    server_name ${HOSTNAME};

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
    }
}
EOF

ln -s "$LANDING_PAGE_PATH" "$LANDING_PAGE_LINK" || true
nginx -t
systemctl daemon-reexec
systemctl restart nginx

# 4) Certbot install (prefer snap; apt can be too old and fail ACME account reg)
if ! command -v certbot >/dev/null 2>&1 || snap list 2>/dev/null | ! grep -q certbot; then
  echo -e "\n* * * Installing Certbot (snap) * * *"
  # snapd must be present
  if ! command -v snap >/dev/null 2>&1; then
    apt update && apt install -y snapd
    systemctl enable --now snapd.socket
    sleep 2
  fi
  snap install core || true
  snap refresh core || true
  snap install --classic certbot
  ln -sf /snap/bin/certbot /usr/bin/certbot
fi

# (Optional) Use staging first to avoid rate limits while debugging:
USE_STAGING=${CERTBOT_STAGING:-0}
STAGING_FLAG=$([[ "$USE_STAGING" = "1" ]] && echo "--staging" || echo "")

echo -e "\n* * * Requesting/renewing certificate via nginx plugin * * *"
# Important: non-interactive + agree-tos + email + hostname
certbot --nginx --non-interactive --agree-tos -m "$EMAIL" -d "$HOSTNAME" $STAGING_FLAG --redirect

# 5) WSS listener (9001) using issued certs
WSS_CONFIG_PATH="/etc/nginx/sites-available/wss-config-nym"
WSS_CONFIG_LINK="/etc/nginx/sites-enabled/wss-config-nym"
[ -L "$WSS_CONFIG_LINK" ] && unlink "$WSS_CONFIG_LINK" || true

cat > "$WSS_CONFIG_PATH" <<EOF
server {
    listen 9001 ssl http2;
    listen [::]:9001 ssl http2;

    server_name ${HOSTNAME};

    ssl_certificate /etc/letsencrypt/live/${HOSTNAME}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${HOSTNAME}/privkey.pem;
    include /etc/letsencrypt/options-ssl-nginx.conf;
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem;

    access_log /var/log/nginx/access.log;
    error_log /var/log/nginx/error.log;

    location /favicon.ico { return 204; access_log off; log_not_found off; }

    location / {
        add_header 'Access-Control-Allow-Origin' '*';
        add_header 'Access-Control-Allow-Credentials' 'true';
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS, HEAD';
        add_header 'Access-Control-Allow-Headers' '*';

        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header X-Forwarded-For \$remote_addr;

        proxy_pass http://localhost:9000;
        proxy_intercept_errors on;
    }
}
EOF

ln -s "$WSS_CONFIG_PATH" "$WSS_CONFIG_LINK" || true
nginx -t
systemctl daemon-reexec
systemctl restart nginx

echo -e "\nDone. If you used staging (CERTBOT_STAGING=1), re-run without it to get a real cert."
