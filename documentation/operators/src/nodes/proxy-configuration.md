# Reverse Proxy & Web Secure Socket

This section will guide you in setting up a reverse proxy for serving `nym-node` HTTP requests and to set up a custom [landing page](../legal/landing-pages.md) for your node.

In later sections, you will be setting up secure websocket (wss) to add additional security and encrypt connections coming to your node. Follow [this guide](#web-secure-socket-setup) for installation.

```admonish info
Since SSL certificates can only be issued for a domain name and not an IP address, it is essential for you to register a new domain name and configure a domain record pointing to your node's IP address
```

## Variables Explanation

This guide contains several variables. Substitute them with your own value, without `<>` brackets. Here is a list of variables we used below.

| Variable              | Description                                                                                 | Syntax example                                            |
| :-------------------- | :------------------------------------------------------------------------------------------ | :-------------------------------------------------------- |
| `<HOSTNAME>`          | Your registered DNS domain, asigned to the VPS with `nym-node`                              | exit-gateway1.squad.nsl                                   |
| `<WSS_PORT>`          | Port listening to WSS, default is `9001`                                                    | 9001                                                      |
| `<YOUR_WELCOME_TEXT>` | Any text you want to show on the landing page                                               | Welcome to Nym Node, operator contact is example@email.me |
| `<LANDING_PAGE_PATH>` | A sub-directory located at `/var/www/<HOSTNAME>` containing html configuration files        | `/var/www/exit-gateway1.squad.nsl`                        |
| `<ID>`                | A local only `nym-node` identifier, specified by flag `--id`, default is `default-nym-node` | alice_super_node                                          |
| `<PATH_TO>`           | Specify a full path to the given file, directory or binary behind this variable             | `/root/src/nym/target/release/`                           |

```admonish warning title=""
The commands in this setup need to be run with root permission. Either add a prefix `sudo` or execute them from a root shell.
```

## Reverse Proxy Setup

```admonish info
This guide was created by a Nym node operator, [Avril 14th](https://avril14th.org) as a part of [Nym Operators Community Counsel](../legal/community-counsel.md), edited by Nym.
```

The following snippet needs  be modified as described below according to the public identity that you may want to show on this public notice, i.e. your graphics and your email.
It would allow you to serve it as a landing page resembling the one proposed by [Tor](https://gitlab.torproject.org/tpo/core/tor/-/raw/HEAD/contrib/operator-tools/tor-exit-notice.html) but with all the changes needed to adhere to the Nym's operators case.



### HTML File Customization

File for html configuration are by convention located at `/var/www/<HOSTNAME>` directory and it's subdirectories. We refer to this directory as `<LANDING_PAGE_PATH>`.

1. Start by creating this directory:
```sh
mkdir -p /var/www/<HOSTNAME>
```

2. Use your own html code or copy the template below to a new file called `index.html` located in `/var/www/<HOSTNAME>` directory.

~~~admonish example collapsible=true title="An example template for `/var/www/<HOSTNAME>/index.html` page"
```html
<!DOCTYPE html>
<html lang="en-US">
<head>
<meta charset="UTF-8">
<title>This is a NYM Exit Gateway</title>
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="icon" type="image/png" href="">
<style>
:root {
  font-family: Consolas, "Ubuntu Mono", Menlo, "DejaVu Sans Mono", monospace;
}
:root{
--background-color: #121726;
--text-color: #f2f2f2;
--link-color: #fb6e4e;
}
html{
background: var(--background-color);
}
body{
margin-left: auto;
margin-right: auto;
padding-left: 5vw;
padding-right: 5vw;
max-width: 1000px;
}
h1{
font-size: 55px;
text-align: center;
color: var(--title-color)
}
p{
color: var(--text-color);
}
p, a{
font-size: 20px;
}
a{
color: var(--link-color);
text-decoration: none;
}
a:hover{
filter: brightness(.8);
text-decoration: underline;
}
.links{
display: flex;
flex-wrap: wrap;
justify-content: space-evenly;
}
.links > a{
margin: 10px;
white-space: nowrap;
}
</style>

</head>
<body>
<main>
<h1>This is a NYM Exit Gateway</h1>
<p style="text-align:center">
<img class="logo" src="<FIXME>">
</p>

<p>
You are most likely accessing this website because you've had some issue with
the traffic coming from this IP. This router is part of the <a
href="https://nymtech.net/">NYM project</a>, which is
dedicated to <a href="https://nymtech.net/about/mission">create</a> outstanding
privacy software that is legally compliant without sacrificing integrity or
having any backdoors.
This router IP should be generating no other traffic, unless it has been
compromised.</p>

<p>
The Nym mixnet is operated by a decentralised community of node operators
and stakers. The Nym mixnet is trustless, meaning that no parts of the system
nor its operators have access to information that might compromise the privacy
of users. Nym software enacts a strict principle of data minimisation and has
no back doors. The Nym mixnet works by encrypting packets in several layers
and relaying those through a multi-layered network called a mixnet,  eventually
letting the traffic exit the Nym mixnet through an exit gateway like this one.
This design makes it very hard for a service to know which user is connecting to it,
since it can only see the IP-address of the Nym exit gateway:</p>

<p style="text-align:center;margin:40px 0">
<svg xmlns="http://www.w3.org/2000/svg" width="500" viewBox="0 0 490.28 293.73" style="width:100%;max-width:600px">
<desc>Illustration showing how a user might connect to a service through the Nym network. The user first sends their data through three daisy-chained encrypted Nym nodes that exist on three different continents. Then the last Nym node in the chain connects to the target service over the normal internet.</desc>
<defs>
<style>
.t{
fill: var(--text-color);
stroke: var(--text-color);
}
</style>
</defs>
<path fill="#6fc8b7" d="M257.89 69.4c-6.61-6.36-10.62-7.73-18.36-8.62-7.97-1.83-20.06-7.99-24.17-.67-3.29 5.85-18.2 12.3-16.87 2.08.92-7.03 11.06-13.28 17-17.37 8.69-5.99 24.97-2.87 26.1-10.28 1.04-6.86-8.33-13.22-8.55-2.3-.38 12.84-19.62 2.24-8.73-6.2 8.92-6.9 16.05-9.02 25.61-6.15 12.37 4.83 25.58-2.05 33.73-.71 12.37-2.01 24.69-5.25 37.39-3.96 13 .43 24.08-.14 37.06.63 9.8 1.58 16.5 2.87 26.37 3.6 6.6.48 17.68-.82 24.3 1.9 8.3 4.24.44 10.94-6.89 11.8-8.79 1.05-23.59-1.19-26.6 1.86-5.8 7.41 10.75 5.68 11.27 14.54.57 9.45-5.42 9.38-8.72 16-2.7 4.2.3 13.93-1.18 18.45-1.85 5.64-19.64 4.47-14.7 14.4 4.16 8.34 1.17 19.14-10.33 12.02-5.88-3.65-9.85-22.04-15.66-21.9-11.06.27-11.37 13.18-12.7 17.52-1.3 4.27-3.79 2.33-6-.63-3.54-4.76-7.75-14.22-12.01-17.32-6.12-4.46-10.75-1.17-15.55 2.83-5.63 4.69-8.78 7.82-7.46 16.5.78 9.1-12.9 15.84-14.98 24.09-2.61 10.32-2.57 22.12-8.81 31.47-4 5.98-14.03 20.12-21.27 14.97-7.5-5.34-7.22-14.6-9.56-23.08-2.5-9.02.6-17.35-2.57-26.2-2.45-6.82-6.23-14.54-13.01-13.24-6.5.92-15.08 1.38-19.23-2.97-5.65-5.93-6-10.1-6.61-18.56 1.65-6.94 5.79-12.64 10.38-18.63 3.4-4.42 17.45-10.39 25.26-7.83 10.35 3.38 17.43 10.5 28.95 8.57 3.12-.53 9.14-4.65 7.1-6.62zm-145.6 37.27c-4.96-1.27-11.57 1.13-11.8 6.94-1.48 5.59-4.82 10.62-5.8 16.32.56 6.42 4.34 12.02 8.18 16.97 3.72 3.85 8.58 7.37 9.3 13.1 1.24 5.88 1.6 11.92 2.28 17.87.34 9.37.95 19.67 7.29 27.16 4.26 3.83 8.4-2.15 6.52-6.3-.54-4.54-.6-9.11 1.01-13.27 4.2-6.7 7.32-10.57 12.44-16.64 5.6-7.16 12.74-11.75 14-20.9.56-4.26 5.72-13.86 1.7-16.72-3.14-2.3-15.83-4-18.86-6.49-2.36-1.71-3.86-9.2-9.86-12.07-4.91-3.1-10.28-6.73-16.4-5.97zm11.16-49.42c6.13-2.93 10.58-4.77 14.61-10.25 3.5-4.28 2.46-12.62-2.59-15.45-7.27-3.22-13.08 5.78-18.81 8.71-5.96 4.2-12.07-5.48-6.44-10.6 5.53-4.13.38-9.2-5.66-8.48-6.12.8-12.48-1.45-18.6-1.73-5.3-.7-10.13-1-15.45-1.37-5.37-.05-16.51-2.23-25.13.87-5.42 1.79-12.5 5.3-16.73 9.06-4.85 4.2.2 7.56 5.54 7.45 5.3-.22 16.8-5.36 20.16.98 3.68 8.13-5.82 18.29-5.2 26.69.1 6.2 3.37 11 4.74 16.98 1.62 5.94 6.17 10.45 10 15.14 4.7 5.06 13.06 6.3 19.53 8.23 7.46.14 3.34-9.23 3.01-14.11 1.77-7.15 8.49-7.82 12.68-13.5 7.14-7.72 16.41-13.4 24.34-18.62zM190.88 3.1c-4.69 0-13.33.04-18.17-.34-7.65.12-13.1-.62-19.48-1.09-3.67.39-9.09 3.34-5.28 7.04 3.8.94 7.32 4.92 7.1 9.31 1.32 4.68 1.2 11.96 6.53 13.88 4.76-.2 7.12-7.6 11.93-8.25 6.85-2.05 12.5-4.58 17.87-9.09 2.48-2.76 7.94-6.38 5.26-10.33-1.55-1.31-2.18-.64-5.76-1.13zm178.81 157.37c-2.66 10.08-5.88 24.97 9.4 15.43 7.97-5.72 12.58-2.02 17.47 1.15.5.43 2.65 9.2 7.19 8.53 5.43-2.1 11.55-5.1 14.96-11.2 2.6-4.62 3.6-12.39 2.76-13.22-3.18-3.43-6.24-11.03-7.7-15.1-.76-2.14-2.24-2.6-2.74-.4-2.82 12.85-6.04 1.22-10.12-.05-8.2-1.67-29.62 7.17-31.22 14.86z"/>
<g fill="none">
<path stroke="#cf63a6" stroke-linecap="round" stroke-width="2.76" d="M135.2 140.58c61.4-3.82 115.95-118.83 151.45-103.33"/>
<path stroke="#cf63a6" stroke-linecap="round" stroke-width="2.76" d="M74.43 46.66c38.15 8.21 64.05 42.26 60.78 93.92M286.65 37.25c-9.6 39.44-3.57 57.12-35.64 91.98"/>
<path stroke="#e4c101" stroke-dasharray="9.06,2.265" stroke-width="2.27" d="M397.92 162.52c-31.38 1.26-90.89-53.54-148.3-36.17"/>
<path stroke="#cf63a6" stroke-linecap="round" stroke-width="2.77" d="M17.6 245.88c14.35 0 14.4.05 28-.03"/>
<path stroke="#e3bf01" stroke-dasharray="9.06,2.265" stroke-width="2.27" d="M46.26 274.14c-17.52-.12-16.68.08-30.34.07"/>
</g>
<g transform="translate(120.8 -35.81)">
<circle cx="509.78" cy="68.74" r="18.12" fill="#240a3b" transform="translate(-93.3 38.03) scale(.50637)"/>
<circle cx="440.95" cy="251.87" r="18.12" fill="#240a3b" transform="translate(-93.3 38.03) scale(.50637)"/>
<circle cx="212.62" cy="272.19" r="18.12" fill="#240a3b" transform="translate(-93.3 38.03) scale(.50637)"/>
<circle cx="92.12" cy="87.56" r="18.12" fill="#240a3b" transform="translate(-93.3 38.03) scale(.50637)"/>
<circle cx="730.88" cy="315.83" r="18.12" fill="#67727b" transform="translate(-93.3 38.03) scale(.50637)"/>
<circle cx="-102.85" cy="282.18" r="9.18" fill="#240a3b"/>
<circle cx="-102.85" cy="309.94" r="9.18" fill="#67727b"/>
</g>
<g class="t">
<text xml:space="preserve" x="-24.76" y="10.37" stroke-width=".26" font-size="16.93" font-weight="700" style="line-height:1.25" transform="translate(27.79 2.5)" word-spacing="0"><tspan x="-24.76" y="10.37">The user</tspan></text>
<text xml:space="preserve" x="150.63" y="196.62" stroke-width=".26" font-size="16.93" font-weight="700" style="line-height:1.25" transform="translate(27.79 2.5)" word-spacing="0"><tspan x="150.63" y="196.62">This server</tspan></text>
<text xml:space="preserve" x="346.39" y="202.63" stroke-width=".26" font-size="16.93" font-weight="700" style="line-height:1.25" transform="translate(27.79 2.5)" word-spacing="0"><tspan x="346.39" y="202.63">Your service</tspan></text>
<text xml:space="preserve" x="34.52" y="249.07" stroke-width=".26" font-size="16.93" font-weight="700" style="line-height:1.25" transform="translate(27.79 2.5)" word-spacing="0"><tspan x="34.52" y="249.07">Nym network link</tspan></text>
<text xml:space="preserve" x="34.13" y="276.05" stroke-width=".26" font-size="16.93" font-weight="700" style="line-height:1.25" transform="translate(27.79 2.5)" word-spacing="0"><tspan x="34.13" y="276.05">Unencrypted link</tspan></text>
<path fill="none" stroke-linecap="round" stroke-width="1.67" d="M222.6 184.1c-2.6-15.27 8.95-23.6 18.43-38.86m186.75 45.61c-.68-10.17-9.4-17.68-18.08-23.49"/>
<path fill="none" stroke-linecap="round" stroke-width="1.67" d="M240.99 153.41c.35-3.41 1.19-6.17.04-8.17m-7.15 5.48c1.83-2.8 4.58-4.45 7.15-5.48"/>
<path fill="none" stroke-linecap="round" stroke-width="1.67" d="M412.43 173.21c-2.2-3.15-2.54-3.85-2.73-5.85m0 0c2.46-.65 3.85.01 6.67 1.24M61.62 40.8C48.89 36.98 36.45 27.54 36.9 18.96M61.62 40.8c.05-2.58-3.58-4.8-5.25-5.26m-2.65 6.04c1.8.54 6.8 1.31 7.9-.78"/>
<path fill="none" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.44" d="M1.22 229.4h247.74v63.1H1.22z"/>
</g>
</svg>
</p>

<p>
<a href="https://nymtech.net/about/mixnet">Read more about how Nym works.</a></p>

<p>
Nym relies on a growing ecosystem of users, developers and researcher partners
aligned with the mission to make sure Nym software is running, remains usable
and solves real problems. While Nym is not designed for malicious computer
users, it is true that they can use the network for malicious ends. This
is largely because criminals and hackers have significantly better access to
privacy and anonymity than do the regular users whom they prey upon. Criminals
can and do build, sell, and trade far larger and more powerful networks than
Nym on a daily basis. Thus, in the mind of this operator, the social need for
easily accessible censorship-resistant private, anonymous communication trumps
the risk of unskilled bad actors, who are almost always more easily uncovered
by traditional police work than by extensive monitoring and surveillance anyway.</p>

<p>
In terms of applicable law, the best way to understand Nym is to consider it a
network of routers operating as common carriers, much like the Internet
backbone. However, unlike the Internet backbone routers, Nym mixnodes do not
contain identifiable routing information about the source of a packet and do
mix the user internet traffic with that of other users, making communications
private and protecting not just the user content but the metadata
(user's IP address, who the user talks to, when, where, from what device and
more) and no single Nym node can determine both the origin and destination
of a given transmission.</p>

<p>
As such, there is little the operator of this Exit Gateway can do to help you
track the connection further. This Exit Gateway maintains no logs of any of the
Nym mixnet traffic, so there is little that can be done to trace either legitimate or
illegitimate traffic (or to filter one from the other).  Attempts to
seize this router will accomplish nothing.</p>

<!-- FIXME: US-Only section. Remove if you are a non-US operator -->
<!--
<p>
Furthermore, this machine also serves as a carrier of email, which means that
its contents are further protected under the ECPA. <a
href="https://www.law.cornell.edu/uscode/text/18/2707">18
USC 2707</a> explicitly allows for civil remedies ($1000/account
<i>plus</i>  legal fees)
in the event of a seizure executed without good faith or probable cause (it
should be clear at this point that traffic with an originating IP address of
FIXME_DNS_NAME should not constitute probable cause to seize the
machine). Similar considerations exist for 1st amendment content on this
machine.</p>
-->
<!-- FIXME: May or may not be US-only. Some non-US tor nodes have in
     fact reported DMCA harassment... -->
<!--
<p>
If you are a representative of a company who feels that this router is being
used to violate the DMCA, please be aware that this machine does not host or
contain any illegal content. Also be aware that network infrastructure
maintainers are not liable for the type of content that passes over their
equipment, in accordance with <a
href="https://www.law.cornell.edu/uscode/text/17/512">DMCA
"safe harbor" provisions</a>. In other words, you will have just as much luck
sending a takedown notice to the Internet backbone providers.
</p>
-->

<p>To decentralise and enable privacy for a broad range of services, this
Exit Gateway adopts an <a href="https://nymtech.net/.wellknown/network-requester/exit-policy.txt">Exit Policy</a>
in accordance with the <a href="https://tornull.org/">Tor Null ‘deny’ list</a>
and the <a href="https://tornull.org/tor-reduced-reduced-exit-policy.php">Tor reduced policy</a>,
which are two established safeguards.
</p>

<p>
That being said, if you still have a complaint about the router, you may email the
 <a href="mailto:>YOUR_EMAIL_ADDRESS>">maintainer</a>. If complaints are related
 to a particular service that is being abused, the maintainer will submit that to the
 NYM Operators Community in order to add it to the Exit Policy cited above.
If approved, that would prevent this router from allowing that traffic to exit through it.
That can be done only on an IP+destination port basis, however. Common P2P ports are already blocked.</p>

<p>
You also have the option of blocking this IP address and others on the Nym network if you so desire.
 The Nym project provides a <a href="https://explorer.nymtech.net/network-components/gateways">
 web service</a> to fetch a list of all IP addresses of Nym Gateway Exit nodes that allow exiting to a
specified IP:port combination. Please be considerate when using these options.</p>

</main>
</body>
</html>
```
~~~

3. Before you save and close the file, make sure to edit the text, especially the information in these points:

- Add your favicon logo on the line:
```
<link rel="icon" type="image/png" href="">
```

- Add your header logo on the line:
```
<img class="logo" src="<FIXME>">
```

- By either setting the URl to the image (if you're hosting it publicly, i.e. on your web server)
```
href="<PATH_TO_YOUR_PUBLIC_URL>"

# and

src="<PATH_TO_YOUR_PUBLIC_URL>"
```

- **or** by adding the image inline as base64 encoded image
```
href="href="data:image/x-icon;base64,AAABAAMA....""

# and

src="href="data:image/x-icon;base64,AAABAAMA....""
```

- Add the email address you're willing to use for being contacted.
```
<a href="mailto:>YOUR_EMAIL_ADDRESS>">maintainer</a>
```

- If you're running the node within the US check the sections marked as `FIXME`, add your DNS name and un-comment those.

4. Save and exit

Now your html page is configured.

### `nym-node` Configuration

When done with the customization, you'll need to make sure your `nym-node` uploads the file and reference to it. This is done by opening your node configuration file located at `~/.nym/nym-nodes/<ID>/config/config.toml` and changing the value of the line `landing_page_assets_path` on the `[http]` section:
```
landing_page_assets_path = '<LANDING_PAGE_PATH>'
```

### Reverse Proxy Configuration

You may set up a [reverse proxy](https://www.nginx.com/resources/glossary/reverse-proxy-server/) in order to serve this landing page with proper SSL and DNS management, i.e. to resolve it to https://<HOSTNAME>.

**Configure Nginx**

1. Install `nginx`:
```sh
sudo apt install nginx
```

1. Setup firewall with `ufw`. `ufw` has three profile pre-configured for `nginx`, we will need the first one for `nym-node`:

- `Nginx Full`: This profile opens both port 80 (normal, unencrypted web traffic) and port 443 (TLS/SSL encrypted traffic)
- `Nginx HTTP`: This profile opens only port 80 (normal, unencrypted web traffic)
- `Nginx HTTPS`: This profile opens only port 443 (TLS/SSL encrypted traffic)

```sh
ufw allow 'Nginx Full'

# you can verify by
ufw status

# possibly reload ufw by
ufw reload
```

3. Disable the default Nginx landing page

```
systemctl status nginx
unlink /etc/nginx/sites-enabled/default
systemctl restart nginx
```

4. Add your endpoint configuration to Nginx by creating file:

```sh
nano /etc/nginx/sites-available/<HOSTNAME>
```
- and changing `<HOSTNAME>` occurrences below with your domain name:

```
server {
    listen 80;
    listen [::]:80;

    # Replace <HOSTNAME> with your domain name
    server_name <HOSTNAME>;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```
This guide assumes that the HTTP port used by you is `8080`. Adjust the configuration accordingly if you have defined
a custom port for your `nym-node` HTTP connections

5. Activate the configuration by creating a symlink to `/etc/nginx/sites-enabled`:

```sh
ln -s /etc/nginx/sites-available/<HOSTNAME> /etc/nginx/sites-enabled
```

6. Test your configuration syntax:

```sh
nginx -t
```

Nginx must report that the config is "ok" and the test was successful.

7. Restart `nginx`:

```sh
systemctl restart nginx
```

8. Get an `SSL` certificate using certbot:

```sh
apt install certbot python3-certbot-nginx
certbot --nginx --non-interactive --agree-tos --redirect -m <YOUR_EMAIL_ADDRESS> -d <HOSTNAME>
```

9. Restart your `nym-node` or if you're running your `nym-node` as a [`systemd` service](configuration.md#systemd), restart your service:
```sh
systemctl daemon-reload && systemctl restart nym-node
```

9. Check for the page being served reading the service logs
```sh
journalctl -u  nym-node.service | grep 8080

# where you should see
... Started NymNodeHTTPServer on 0.0.0.0:8080
```

Now your `nginx` should be configured, up and running. Test it by inserting your `<HOSTNAME>` as a URL in a browser.


## Web Secure Socket Setup

This section assumes that you have already configured a reverse proxy and have set it up to work over https. If not, head over to [the reverse proxy section](#reverse-proxy-configuration) to configure it.

We strongly recommend node operators to configure secure web sockets on their nodes. This will provide clients a more secure way to connect to your node.

You can read more about *Secure Socket Layer* (SSL) in [here](https://www.geeksforgeeks.org/secure-socket-layer-ssl/).


Remember that there may be some unique variables and customization depending on the way your reverse proxy is setup which you may have to adjust when configuring WSS to ensure correct functionality

```admonish tip
To see description of used variables (noted in `<>` brackets), scroll to the top of this page, chapter [*Variables Explanation*](#variables-explanation).
```

#### Firewall configuration

Make sure to open all [needed ports](vps-setup.md#configure-your-firewall), adding your `<WSS_PORT>`:

```sh
ufw allow <WSS_PORT>/tcp

# for example
# ufw allow 9001/tcp
```

#### WSS configuration

This section assumes that you have already configured a reverse proxy and have set it up to work over https. If not, head over to [the reverse proxy section](#reverse-proxy) to configure it.

1. Create a new Nginx configuration file called `/etc/nginx/sites-available/wss-config-nym` and paste the block below. Don't forget to insert your correct values.

~~~admonish example collapsible=true title="Site configuration `/etc/nginx/sites-available/wss-config-nym`"
```bash
#####################################################
# EXCHANGE ALL <HOSTNAME> & <WSS_PORT> VARIABLES ! #
####################################################

server {
    listen <WSS_PORT> ssl http2;
    listen [::]:<WSS_PORT> ssl http2;

    server_name <HOSTNAME>;

    ssl_certificate /etc/letsencrypt/live/<HOSTNAME>/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/<HOSTNAME>/privkey.pem;
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
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header X-Forwarded-For $remote_addr;

        proxy_pass http://localhost:9000;
        proxy_intercept_errors on;
    }
}
```
~~~

4. Activate the configuration by creating a simlink to `/etc/nginx/sites-enabled`:
```sh
ln -s /etc/nginx/sites-available/wss-config-nym /etc/nginx/sites-enabled
```

5. Test your configuration syntax:
```sh
nginx -t
```

6. Restart `nginx`:
```sh
systemctl restart nginx

```

7. Finally, configure your `nym-node` to announce the port you have setup. This is done by opening your node configuration file located at `~/.nym/nym-nodes/<ID>/config/config.toml` and changing the value of the line `announce_wss_port` in the `[entry_gateway]` section:

```
announce_wss_port =  <WSS_PORT>

# example
# announce_wss_port = 9001
```

8. Restart your `nym-node` :
```sh
systemctl restart nym-node
```

Your `nym-node` should be configured to run over WSS now. Test it using the steps in the chapter [below](#test-wss-setup).

### Test WSS Setup

You can do a few quick checks to test that your installation worked out and your `nym-node` is running correctly using WSS:

- Check out connection with `wscat` from another (local) machine:
```sh
# install
sudo apt install node-ws

# run
wscat -c wss://<HOSTNAME>:<WSS_PORT>
```

- Check Swagger API of your node using the hostname: `https://<HOSTNAME>/api/v1/swagger/#/`
