#!/usr/bin/env bash
set -euo pipefail

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
HTTPS_CONF="${SITES_AVAIL}/${HOSTNAME}-ssl"
WSS_CONF="${SITES_AVAIL}/wss-config-nym"

echo
echo "* * * starting clean nginx configuration for landing page, reverse proxy and wss * * *"

################################################################################
# step 1: clean all previous configs
################################################################################

echo "cleaning existing nginx configuration"

# remove default nginx config
[[ -L "${SITES_EN}/default" ]] && unlink "${SITES_EN}/default" || true

# remove domain symlinks
rm -f "${SITES_EN}/${HOSTNAME}"       || true
rm -f "${SITES_EN}/${HOSTNAME}-ssl"   || true
rm -f "${SITES_EN}/wss-config-nym"    || true

# remove old configs
rm -f "${HTTP_CONF}"                  || true
rm -f "${HTTPS_CONF}"                 || true
rm -f "${WSS_CONF}"                   || true

################################################################################
# step 2: landing page
################################################################################

mkdir -p "${WEBROOT}"

if ! curl -fsSL \
  https://raw.githubusercontent.com/nymtech/nym/develop/scripts/nym-node-setup/landing-page.html \
  -o "${WEBROOT}/index.html"; then

  cat > "${WEBROOT}/index.html" <<'EOF'
<!DOCTYPE html>
<html>
<head><title>nym node</title></head>
<body style="font-family:sans-serif;text-align:center;padding:2em;">
<h1>nym exit gateway</h1>
<p>this is a nym exit gateway.</p>
</body>
</html>
EOF

fi

echo "landing page at ${WEBROOT}/index.html"

################################################################################
# step 3: HTTP :80 config
################################################################################

cat > "${HTTP_CONF}" <<EOF
server {
    listen 80;
    listen [::]:80;

    server_name ${HOSTNAME};

    # ACME challenge
    location /.well-known/acme-challenge/ {
        root /var/www/letsencrypt;
    }

    # reverse proxy
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

ln -sf "${HTTP_CONF}" "${SITES_EN}/${HOSTNAME}"

nginx -t
systemctl reload nginx

################################################################################
# step 4: obtain certificate
################################################################################

apt-get update -y >/dev/null 2>&1 || true
apt-get install -y certbot python3-certbot-nginx >/dev/null 2>&1 || true

echo "requesting let's encrypt certificate for ${HOSTNAME}"

certbot --nginx --non-interactive --agree-tos \
  --reuse-key \
  -m "${EMAIL}" -d "${HOSTNAME}" --redirect || true

################################################################################
# step 5: HTTPS and WSS configs
################################################################################

if [[ -s "/etc/letsencrypt/live/${HOSTNAME}/fullchain.pem" ]]; then
  echo "certificate detected, creating https and wss configs"

  # HTTPS 443
  cat > "${HTTPS_CONF}" <<EOF
server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;

    server_name ${HOSTNAME};

    ssl_certificate /etc/letsencrypt/live/${HOSTNAME}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${HOSTNAME}/privkey.pem;

    root ${WEBROOT};
    index index.html;

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

  ln -sf "${HTTPS_CONF}" "${SITES_EN}/${HOSTNAME}-ssl"

  # WSS 9001
  cat > "${WSS_CONF}" <<EOF
server {
    listen 9001 ssl http2;
    listen [::]:9001 ssl http2;

    server_name ${HOSTNAME};

    ssl_certificate /etc/letsencrypt/live/${HOSTNAME}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${HOSTNAME}/privkey.pem;

    access_log /var/log/nginx/access.log;
    error_log  /var/log/nginx/error.log;

    location / {
        add_header Access-Control-Allow-Origin '*' always;
        add_header Access-Control-Allow-Methods 'GET, POST, OPTIONS, HEAD' always;
        add_header Access-Control-Allow-Headers '*' always;

        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header X-Forwarded-For \$remote_addr;

        proxy_pass http://127.0.0.1:9000;
    }
}
EOF

  ln -sf "${WSS_CONF}" "${SITES_EN}/wss-config-nym"

else
  echo "certificate missing, skipping https and wss configs"
fi

################################################################################
# step 6: reload nginx + summary
################################################################################

nginx -t
systemctl reload nginx

echo "done."
echo "http  : http://${HOSTNAME}"

if [[ -s "/etc/letsencrypt/live/${HOSTNAME}/fullchain.pem" ]]; then
  echo "https : https://${HOSTNAME}"
  echo "wss   : wss://${HOSTNAME}:9001"
else
  echo "https not active yet (no cert)"
fi
