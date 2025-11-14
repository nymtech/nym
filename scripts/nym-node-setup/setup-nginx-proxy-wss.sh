#!/usr/bin/env bash
set -euo pipefail

if [[ "$(id -u)" -ne 0 ]]; then
  echo "This script must be run as root."
  exit 1
fi

# load env
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

HTTP_CONF="${SITES_AVAIL}/${HOSTNAME}"
WSS_CONF="${SITES_AVAIL}/wss-config-nym"

echo
echo "* * * Starting nginx configuration for landing page, reverse proxy and WSS * * *"

###############################################################################
# step 1: ensure landing page exists (local fetch -> github -> template)
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
# step 2: remove default site and old configs, restart nginx
###############################################################################

echo "Cleaning existing nginx configuration"

# remove default nginx site
[[ -L "${SITES_EN}/default" ]] && unlink "${SITES_EN}/default" || true

# optional: remove default available config if present
rm -f /etc/nginx/sites-available/default || true

# remove old vhosts for this domain
rm -f "${SITES_EN}/${HOSTNAME}"     || true
rm -f "${SITES_EN}/${HOSTNAME}-ssl" || true
rm -f "${SITES_EN}/wss-config-nym"  || true

rm -f "${HTTP_CONF}" || true
rm -f "${WSS_CONF}"  || true

systemctl restart nginx || systemctl start nginx

###############################################################################
# step 3: create basic HTTP config like manual flow (80 -> 8080)
###############################################################################

cat > "${HTTP_CONF}" <<EOF
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

ln -sf "${HTTP_CONF}" "${SITES_EN}/${HOSTNAME}"

nginx -t
systemctl daemon-reload
systemctl restart nginx

###############################################################################
# step 4: install certbot and obtain certificate (letsencrypt)
###############################################################################

apt-get update -y >/dev/null 2>&1 || true
apt-get install -y certbot python3-certbot-nginx >/dev/null 2>&1 || true

echo "Requesting Let's Encrypt certificate for ${HOSTNAME}"

certbot --nginx --non-interactive --agree-tos --redirect --reuse-key \
  -m "${EMAIL}" -d "${HOSTNAME}" || true

###############################################################################
# step 5: create WSS 9001 config using certbot-generated certs
###############################################################################

if [[ -s "/etc/letsencrypt/live/${HOSTNAME}/fullchain.pem" ]]; then
  echo "Certificate detected, creating WSS config"

  cat > "${WSS_CONF}" <<EOF
server {
    listen 9001 ssl http2;
    listen [::]:9001 ssl http2;

    server_name ${HOSTNAME};

    ssl_certificate /etc/letsencrypt/live/${HOSTNAME}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${HOSTNAME}/privkey.pem;
    include /etc/letsencrypt/options-ssl-nginx.conf;
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem;

    access_log /var/log/nginx/access.log;
    error_log  /var/log/nginx/error.log;

    location /favicon.ico {
        return 204;
        access_log     off;
        log_not_found  off;
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

  ln -sf "${WSS_CONF}" "${SITES_EN}/wss-config-nym"

  nginx -t
  systemctl daemon-reload
  systemctl restart nginx
else
  echo "Certificate missing, skipping WSS config"
fi

###############################################################################
# step 6: summary
###############################################################################

echo "done."
echo "http  : http://${HOSTNAME}"
if [[ -s "/etc/letsencrypt/live/${HOSTNAME}/fullchain.pem" ]]; then
  echo "https : https://${HOSTNAME}"
  echo "wss   : wss://${HOSTNAME}:9001"
else
  echo "https not active yet (no cert)"
fi
