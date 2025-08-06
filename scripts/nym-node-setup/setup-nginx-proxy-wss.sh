echo "Starting nginx configuration for landing page, reverse proxy and web secure socket"
echo "Creating a landing page stored at /var/www/'$HOSTNAME'"

mkdir -p /var/www/"${HOSTNAME}" && cp landing-page.html /var/www/"${HOSTNAME}"/index.html

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
