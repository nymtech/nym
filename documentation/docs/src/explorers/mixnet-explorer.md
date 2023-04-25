# Mixnet Explorer

The Nym Network Explorer lets you explore the Nym network. We have open-sourced the explorer so that anyone can run an instance of it, further decentralising the network! 

### Prerequisites
- `git`

```
sudo apt update
sudo apt install git
```

Verify `git` is installed with:

```
git version
# Should return: git version X.Y.Z
```

- (Debian/Ubuntu) `pkg-config`, `build-essential`, `libssl-dev`, `curl`, `jq`

```
sudo apt update
sudo apt install pkg-config build-essential libssl-dev curl jq
```

- `NodeJS` (use `nvm install` to automatically install the correct version) and `npm`

- `Rust & cargo >= {{minimum_rust_version}}`

We recommend using the [Rust shell script installer](https://www.rust-lang.org/tools/install). Installing cargo from your package manager (e.g. `apt`) is not recommended as the packaged versions are usually too old.

If you really don't want to use the shell script installer, the [Rust installation docs](https://forge.rust-lang.org/infra/other-installation-methods.html) contain instructions for many platforms.


### Local Development
Complete the steps in the [building nym](../binaries/building-nym.md) section, before `cd`-ing into `nym/explorer`. 

Start a development server with hot reloading running on `http://localhost:3000` with the following commands from inside the `explorer` directory:

```
nvm install # install relevant nodejs and npm versions 
npm install
npm run start
```

`eslint` and `prettier` are already configured.

You can lint the code by running:

```
npm run lint
```

> This command will only **show** linting errors and will not fix them!
 
To fix all linting errors automatically run:

```
npm run lint:fix
```

Please see the development docs in `explorer/docs` for more information on the structure and design of this app.

### Deployment
Complete the steps in the [building nym](../binaries/building-nym.md) section, before `cd`-ing into `nym/explorer`. 

> The Network Explorer should be run on a machine with at least 4GB of RAM - the build process might fail if run on a less powerful machine. 

#### Building the Explorer UI 
Build the UI with these commands from within the `explorer` directory:

```
nvm install # install relevant nodejs and npm versions 
npm install
npm run build
```

The output will be in the `dist` directory. 

This can then be either served directly from the `nym` directory, or from its own directory if you wish. See the template nginx config below for more on how to host this. 

#### Building the Explorer API
The Explorer API was built in the previous step with `cargo build`. 

### Automating the explorer with systemd
You will most likely want to automate the Explorer-API restarting if your server reboots. Below is a systemd unit file to place at `/etc/systemd/system/nym-explorer-api.service`:

```ini
[Unit]
Description=Nym Explorer API (1.1.0)
StartLimitIntervalSec=350
StartLimitBurst=10

[Service]
User=nym
Type=simple
Environment="API_STATE_FILE=/home/nym/network-explorer/explorer-api-state.json"
Environment="GEO_IP_SERVICE_API_KEY=c69155d0-25f6-11ec-80bc-75e5dbd322c3"
ExecStart=explorer/api/location
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
```

Proceed to start it with:

```
systemctl daemon-reload # to pickup the new unit file
systemctl enable nymd   # to enable the service
systemctl start nymd    # to actually start the service
journalctl -f           # to monitor system logs showing the service start
```

### Installing and configuring nginx for HTTPS
#### Setup
[Nginx](https://www.nginx.com/resources/glossary/nginx/#:~:text=NGINX%20is%20open%20source%20software,%2C%20media%20streaming%2C%20and%20more.&text=In%20addition%20to%20its%20HTTP,%2C%20TCP%2C%20and%20UDP%20servers.) is an open source software used for operating high-performance web servers. It allows us to set up reverse proxying on our validator server to improve performance and security.

Install `nginx` and allow the 'Nginx Full' rule in your firewall:

```
sudo ufw allow 'Nginx Full'
```

Check nginx is running via systemctl:

```
systemctl status nginx
```

Which should return:

```
● nginx.service - A high performance web server and a reverse proxy server
   Loaded: loaded (/lib/systemd/system/nginx.service; enabled; vendor preset: enabled)
   Active: active (running) since Fri 2018-04-20 16:08:19 UTC; 3 days ago
     Docs: man:nginx(8)
 Main PID: 2369 (nginx)
    Tasks: 2 (limit: 1153)
   CGroup: /system.slice/nginx.service
           ├─2369 nginx: master process /usr/sbin/nginx -g daemon on; master_process on;
           └─2380 nginx: worker process
```

#### Configuration
Replace the default nginx configuration at `/etc/nginx/sites-available/` with: 

```
server {
  listen 80;
  listen [::]:80;
  server_name domain;
  root html_location;
  location / {
    try_files /$uri /$uri/index.html /index.html =404;
  }

  location /api {
      proxy_pass http://127.0.0.1:8000;
		  rewrite /api/(.*) /$1  break;
                  proxy_set_header  X-Real-IP $remote_addr;
                  proxy_set_header  Host $host;
                  proxy_set_header  X-Real-IP $remote_addr;
  }
}
```

Followed by:

```
sudo apt install certbot nginx python3
certbot --nginx -d nym-validator.yourdomain.com -m you@yourdomain.com --agree-tos --noninteractive --redirect
```

```admonish caution
If using a VPS running Ubuntu 20: replace `certbot nginx python3` with `python3-certbot-nginx`
```

### Configure your firewall
The following commands will allow you to set up a firewall using `ufw`.

```
# check if you have ufw installed
ufw version
# if it is not installed, install with
sudo apt install ufw -y
# enable ufw
sudo ufw enable
# check the status of the firewall
sudo ufw status
```

Now open the ports: 

```
sudo ufw allow 22,80,443/tcp
# check the status of the firewall
sudo ufw status
```