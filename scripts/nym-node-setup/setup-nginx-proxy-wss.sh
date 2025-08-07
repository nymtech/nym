echo "Starting nginx configuration for landing page, reverse proxy and web secure socket"
echo "Creating a landing page stored at /var/www/'$HOSTNAME'"

mkdir -p /var/www/"${HOSTNAME}" && cp ./landing-page.html /var/www/"${HOSTNAME}"/index.html

systemctl status nginx
unlink /etc/nginx/sites-enabled/default
systemctl restart nginx

echo "Creating a nxing configuration file stored at /etc/nginx/sites-available/'$HOSTNAME'"

cat > /etc/nginx/sites-available/"${HOSTNAME}" <<EOF
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

cat /etc/nginx/sites-available/"${HOSTNAME}"

sleep 2

ln -s /etc/nginx/sites-available/"${HOSTNAME}" /etc/nginx/sites-enabled && \
nginx -t

systemctl daemon-reload && systemctl restart nginx

echo "Installing certbot"

apt install certbot python3-certbot-nginx -y && \
certbot --nginx --non-interactive --agree-tos --redirect -m $EMAIL -d $HOSTNAME

echo "Setting up web secure socket configuration stored at /etc/nginx/sites-available/wss-config-nym"

cat > /etc/nginx/sites-available/wss-config-nym <<EOF
server {
    listen 9001 ssl http2;
    listen [::]:9001 ssl http2;

    server_name ${HOSTNAME};

    ssl_certificate /etc/letsencrypt/live/${HOSTNAME}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/${HOSTNAME}/privkey.pem;
    include /etc/letsencrypt/options-ssl-nginx.conf; # managed by Certbot
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem; # managed by Certbot

    access_log /var/log/nginx/access.log;
    error_log /var/log/nginx/error.log;

    # Ignore favicon requests
    location /favicon.ico {
        return 204;
        access_log     off;
        log_not_found  off;
    }

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

cat /etc/nginx/sites-available/wss-config-nym

sleep 2

ln -s /etc/nginx/sites-available/wss-config-nym /etc/nginx/sites-enabled && \
nginx -t

systemctl daemon-reload && systemctl restart nginx














#!/bin/bash

# Landing Page setup
LANDING_PAGE_PATH="/etc/nginx/sites-available/${HOSTNAME}"
LANDING_PAGE_LINK="/etc/nginx/sites-enabled/${HOSTNAME}"

echo "Starting nginx configuration for landing page, reverse proxy and web secure socket"
echo "Creating a landing page stored at /var/www/'$HOSTNAME'"
mkdir -p /var/www/"${HOSTNAME}" && cp ./landing-page.html /var/www/"${HOSTNAME}"/index.html

# Show nginx status
systemctl status nginx

# Unlink default if exists
[ -L /etc/nginx/sites-enabled/default ] && unlink /etc/nginx/sites-enabled/default

# Handle existing landing config
if [[ -f "$LANDING_PAGE_PATH" ]]; then
  echo "Landing page config exists at $LANDING_PAGE_PATH"
  echo "Choose what to do:"
  echo "1) Overwrite"
  echo "2) Backup and create new"
  echo "3) Cancel"
  read -rp "Press 1, 2, or 3 and enter: " choice
  case "$choice" in
    1) echo "Overwriting..." ;;
    2) cp "$LANDING_PAGE_PATH" "${LANDING_PAGE_PATH}.bak.$(date +%s)" && echo "Backup created." ;;
    3) echo "Cancelled by user."; exit 0 ;;
    *) echo "Invalid choice. Aborting."; exit 1 ;;
  esac
fi

# Remove existing symlink if any
[ -L "$LANDING_PAGE_LINK" ] && unlink "$LANDING_PAGE_LINK"

# Write landing page config
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

ln -s "$LANDING_PAGE_PATH" "$LANDING_PAGE_LINK"
nginx -t
systemctl daemon-reexec && systemctl restart nginx

# Setup SSL certbot
echo "Installing SSL certbot"
apt install certbot python3-certbot-nginx -y
certbot --nginx --non-interactive --agree-tos --redirect -m "$EMAIL" -d "$HOSTNAME"

# Create Web Secure Socket config
WSS_CONFIG_PATH="/etc/nginx/sites-available/wss-config-nym"
WSS_CONFIG_LINK="/etc/nginx/sites-enabled/wss-config-nym"

if [[ -f "$WSS_CONFIG_PATH" ]]; then
  echo "Web Secure Scoket config already exists at $WSS_CONFIG_PATH"
  echo "Choose what to do:"
  echo "1) Overwrite"
  echo "2) Backup and create new"
  echo "3) Cancel"
  read -rp "Press 1, 2, or 3 and enter: " wss_choice
  case "$wss_choice" in
    1) echo "Overwriting..." ;;
    2) cp "$WSS_CONFIG_PATH" "${WSS_CONFIG_PATH}.bak.$(date +%s)" && echo "Backup created." ;;
    3) echo "Cancelled by user."; exit 0 ;;
    *) echo "Invalid choice. Aborting."; exit 1 ;;
  esac
fi

[ -L "$WSS_CONFIG_LINK" ] && unlink "$WSS_CONFIG_LINK"

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

    location /favicon.ico {
        return 204;
        access_log off;
        log_not_found off;
    }

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

ln -s "$WSS_CONFIG_PATH" "$WSS_CONFIG_LINK"
nginx -t
systemctl daemon-reexec && systemctl restart nginx
