# NymConnect Beta (GUI)

NymConnect is a one-button GUI application that wraps around the `nym-socks5-client` for proxying application traffic through the Mixnet. 

You can watch our video on getting started with NymConnect: 

<iframe width="700" height="400" src="https://www.youtube.com/embed/quj8H2qeOwY" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" allowfullscreen></iframe>

Install NymConnect and select an application that you want to privacy-enhance from the dropdown menu. For now, NymConnect can be used with Electrum Wallet, Keybase, desktop Telegram and Blockstream Green. Configure these to run via a SOCKS5 proxy and send their data through the Nym mixnet!

**Please note that NymConnect is currently released in beta. Please report bugs via Github**. 

## Usage instuctions 
* [Download](https://github.com/nymtech/nym/releases/) and install NymConnect.
* Select your service provider from the dropdown menu.
* Click `connect` - NymConnect will connect to a service provider and its SOCKS Proxy (IP) and Port will be displayed.
* Click on IP or Port to copy their values to the clipboard.
* Go to your app settings and look for the network/proxy settings. Select `running via SOCKS5 proxy` and paste the IP and Port values given by NymConnect.

Your traffic from that application will now run through the mixnet for privacy and unlinkability!


