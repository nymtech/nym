#!/usr/bin/env bash
set -euo pipefail

# nginx reverse-proxy + WSS setup for a Nym exit gateway.
#
# This mirrors the Ansible role at ansible/nym-node/roles/nginx exactly:
#   - HTTP vhost (port 80): serves ACME challenge + 301 redirect to HTTPS
#   - own SSL options snippet (does NOT rely on certbot-generated files)
#   - certbot certonly --nginx (obtain only; never lets certbot rewrite vhosts)
#   - HTTPS vhost (443): reverse proxy to nym-node API on 127.0.0.1:8080
#   - WSS vhost (9001): proxy to 127.0.0.1:9000 with CORS + upgrade headers
# SSL/WSS vhosts are only enabled once a certificate actually exists.

if [[ "$(id -u)" -ne 0 ]]; then
  echo "This script must be run as root."
  exit 1
fi

# --- load env (matches the CLI: ENV_FILE, else ./env.sh) ---
if [[ -n "${ENV_FILE:-}" && -f "${ENV_FILE}" ]]; then
  set -a; . "${ENV_FILE}"; set +a
elif [[ -f "./env.sh" ]]; then
  set -a; . ./env.sh; set +a
fi

: "${HOSTNAME:?HOSTNAME not set}"
: "${EMAIL:?EMAIL not set}"

export DEBIAN_FRONTEND=noninteractive

WEBROOT="/var/www/${HOSTNAME}"
SITES_AVAIL="/etc/nginx/sites-available"
SITES_EN="/etc/nginx/sites-enabled"
SNIPPETS="/etc/nginx/snippets"

HTTP_CONF="${SITES_AVAIL}/${HOSTNAME}"
SSL_CONF="${SITES_AVAIL}/${HOSTNAME}-ssl"
WSS_CONF="${SITES_AVAIL}/nym-wss-config"     # matches Ansible role filename
SSL_SNIPPET="${SNIPPETS}/nym-ssl-options.conf"

echo
echo "* * * Starting nginx configuration (landing page, reverse proxy, WSS) * * *"

# --- ensure certbot present (role installs nginx + certbot + plugin) ---
apt-get update -y >/dev/null 2>&1 || true
apt-get install -y certbot python3-certbot-nginx >/dev/null 2>&1 || true

###############################################################################
# step 1: SSL options snippet (own defaults, not certbot's)
###############################################################################
mkdir -p "${SNIPPETS}"
cat > "${SSL_SNIPPET}" <<'EOF'
ssl_session_cache shared:NYMSSL:10m;
ssl_session_timeout 1d;
ssl_session_tickets off;

ssl_protocols TLSv1.2 TLSv1.3;
ssl_prefer_server_ciphers off;

# Reasonable modern cipher set (works across Ubuntu nginx builds)
ssl_ciphers "ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305";

# OCSP stapling is nice but can break if resolver isn't set; keep minimal here.
EOF

###############################################################################
# step 2: landing page (local fetch -> github -> minimal fallback)
###############################################################################
mkdir -p "${WEBROOT}"

SCRIPT_DIR="$(dirname "${ENV_FILE:-./env.sh}")"
LOCAL_FETCHED_PAGE="${SCRIPT_DIR}/landing-page.html"

if [[ -s "${LOCAL_FETCHED_PAGE}" ]]; then
  cp "${LOCAL_FETCHED_PAGE}" "${WEBROOT}/index.html"
elif curl -fsSL \
  https://raw.githubusercontent.com/nymtech/nym/develop/scripts/nym-node-setup/landing-page.html \
  -o "${WEBROOT}/index.html"; then
  :
else
  cat > "${WEBROOT}/index.html" <<EOF
<!DOCTYPE html>
<html>
<head><title>nym node</title></head>
<body style="font-family:sans-serif;text-align:center;padding:2em;">
<h1>nym exit gateway</h1>
<p>this is a nym exit gateway.</p>
<p>Operator contact: <a href="mailto:${EMAIL}">${EMAIL}</a></p>
</body>
</html>
EOF
fi
echo "Landing page at ${WEBROOT}/index.html"

###############################################################################
# step 3: clean existing config for this host + default site
###############################################################################
echo "Cleaning existing nginx configuration"
[[ -L "${SITES_EN}/default" ]] && unlink "${SITES_EN}/default" || true
rm -f "${SITES_AVAIL}/default" || true
rm -f "${SITES_EN}/${HOSTNAME}" "${SITES_EN}/${HOSTNAME}-ssl" "${SITES_EN}/nym-wss-config" || true
# also drop legacy filename from older script versions
rm -f "${SITES_EN}/wss-config-nym" "${SITES_AVAIL}/wss-config-nym" || true

###############################################################################
# step 4: HTTP vhost (ACME challenge + redirect to HTTPS) - always enabled
###############################################################################

CERT_EXISTS=false
[[ -s "/etc/letsencrypt/live/${HOSTNAME}/fullchain.pem" ]] && CERT_EXISTS=true

cat > "${HTTP_CONF}" <<EOF
server {
    listen 80;
    listen [::]:80;

    server_name ${HOSTNAME};

    root ${WEBROOT};
    index index.html;

    location ^~ /.well-known/acme-challenge/ {
        default_type "text/plain";
        try_files \$uri =404;
    }

    location / {
$( $CERT_EXISTS && echo "        return 301 https://\$host\$request_uri;" || echo "        try_files \$uri /index.html;" )
    }
}
EOF
ln -sf "${HTTP_CONF}" "${SITES_EN}/${HOSTNAME}"

# nginx must be up for the ACME http-01 challenge
nginx -t
systemctl enable nginx >/dev/null 2>&1 || true
systemctl restart nginx || systemctl start nginx

###############################################################################
# step 5: obtain certificate (certonly - never lets certbot edit vhosts)
###############################################################################
echo "Requesting Let's Encrypt certificate for ${HOSTNAME}"
certbot certonly --nginx \
  --non-interactive --agree-tos --keep-until-expiring \
  -m "${EMAIL}" -d "${HOSTNAME}" || true

###############################################################################
# step 6: HTTPS + WSS vhosts - only if the cert now exists
###############################################################################
if [[ -s "/etc/letsencrypt/live/${HOSTNAME}/fullchain.pem" ]]; then
  echo "Certificate detected, enabling HTTPS and WSS vhosts"

  # HTTPS vhost (443) -> nym-node API 8080
  cat > "${SSL_CONF}" <<EOF
server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;

    server_name ${HOSTNAME};

    ssl_certificate     /etc/letsencrypt/live/${HOSTNAME}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${HOSTNAME}/privkey.pem;
    include ${SSL_SNIPPET};

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
    }
}
EOF
  ln -sf "${SSL_CONF}" "${SITES_EN}/${HOSTNAME}-ssl"

  # WSS vhost (9001) -> clients port 9000
  cat > "${WSS_CONF}" <<EOF
server {
    listen 9001 ssl http2;
    listen [::]:9001 ssl http2;

    server_name ${HOSTNAME};

    ssl_certificate     /etc/letsencrypt/live/${HOSTNAME}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${HOSTNAME}/privkey.pem;
    include ${SSL_SNIPPET};

    access_log /var/log/nginx/access.log;
    error_log  /var/log/nginx/error.log;

    location /favicon.ico {
        return 204;
        access_log off;
        log_not_found off;
    }

    location / {
        add_header 'Access-Control-Allow-Origin' '*' always;
        add_header 'Access-Control-Allow-Credentials' 'true' always;
        add_header 'Access-Control-Allow-Methods' 'GET, POST, OPTIONS, HEAD' always;
        add_header 'Access-Control-Allow-Headers' '*' always;

        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header X-Forwarded-For \$remote_addr;

        proxy_pass http://localhost:9000;
        proxy_intercept_errors on;
    }
}
EOF
  ln -sf "${WSS_CONF}" "${SITES_EN}/nym-wss-config"

  nginx -t
  systemctl restart nginx
else
  echo "Certificate missing, HTTPS and WSS vhosts NOT enabled (HTTP only)"
fi

###############################################################################
# step 7: summary
###############################################################################
echo "done."
echo "http  : http://${HOSTNAME}"
if [[ -s "/etc/letsencrypt/live/${HOSTNAME}/fullchain.pem" ]]; then
  echo "https : https://${HOSTNAME}"
  echo "wss   : wss://${HOSTNAME}:9001"
else
  echo "https not active yet (no cert)"
fi