#!/usr/bin/python

import os
import subprocess

class NodeSetupCLI:

    def __init__(self):
        self.branch = "feature/node-setup-cli"
        self.welcome_message = self.print_welcome_message()
        self.mode = self.prompt_mode()
        self.prereqs_install_sh = self.fetch_script("prereqs_install_sh")
        self.env_vars_install_sh = self.fetch_script("env_vars_install_sh")
        self.node_install_sh = self.fetch_script("node_install_sh")
        self.landing_page_html = self._check_gwx_mode() and self.fetch_script("landing_page_html")
        self.nginx_proxy_wss_sh = self._check_gwx_mode() and self.fetch_script("nginx_proxy_wss_sh")

    def print_welcome_message(self):
        msg = """
        \nWelcome to NymNodeCLI, an interactive tool to download, install, setup and run nym-node. \
        \n\n================================= \
        \nBefore you begin, make sure that: \
        \n================================= \
        \n- You run this setup on Debian based Linux (ie Ubuntu) \
        \n- You meet minimal requirements: https://nym.com/docs/operators/nodes \
        \n- You agree with Operators Terms & Conditions: https://nym.com/operators-validators-terms \
        \n- You have Nym wallet with at least 101 NYM: https://nym.com/docs/operators/nodes/preliminary-steps/wallet-preparation \
        \n- In case of Gateway behind reverse proxy, you have A and AAAA DNS record pointing to this IP and propagated \
        \n\nTo confirm and continue, write "YES" and press enter: \
        """
        confirmation = input(msg)
        if confirmation.upper() == "YES":
            pass
        else:
            print("Without confirming the points above, we cannot continue.")
            exit(1)



# Build the command
# nym_node_path = os.path.expanduser("~/nym-binaries/nym-node")
# cmd = [nym_node_path, "run", "--mode", mode]

#try:
#    subprocess.run(cmd, check=True)
#except subprocess.CalledProcessError as e:
#    print(f"Command failed with error code {e.returncode}")

    def prompt_mode(self):
        mode = input("\
                \nEnter the mode you want to run nym-node in: \
                \n1) mixnode \
                \n2) entry-gateway \
                \n3) exit-gateway \
                \nPress: 1, 2 or 3 and enter: \
                ").strip()
        if mode == "1" or mode == "mixnode":
            mode = "mixnode"
        elif mode == "2" or mode == "entry-gateway":
            mode = "entry-gateway"
        elif mode == "3" or mode == "exit-gateway":
            mode = "exit-gateway"
        else:
            print("Only numbers 1, 2 or 3 are accepted.")
            exit(1)
        os.environ["NYM_MODE"] = mode
        return mode

    def _return_script_url(self, script_init_name):
        scripts_urls = {
                "prereqs_install_sh": f"https://raw.github.com/nymtech/nym/raw/refs/heads/{self.branch}/scripts/nym-node-setup/nym-node-prereqs-install.sh",
                "env_vars_install_sh": f"https://raw.githubusercontent.com/nymtech/nym/refs/heads/{self.branch}/scripts/nym-node-setup/setup-env-vars.sh",
                "node_install_sh": f"https://raw.github.com/nymtech/nym/raw/refs/heads/{self.branch}/scripts/nym-node-setup/nym-node-install.sh",
                "service_config_sh": f"https://raw.github.com/nymtech/nym/raw/refs/heads/{self.branch}/scripts/nym-node-setup/setup-systemd-service-file.sh",
                "nginx_proxy_wss_sh": f"https://raw.github.com/nymtech/nym/raw/refs/heads/{self.branch}/scripts/nym-node-setup/setup-nginx-proxy-wss.sh",
                "landing_page_html": f"https://raw.github.com/nymtech/nym/raw/refs/heads/{self.branch}/scripts/nym-node-setup/landing-page.html"

                }
        return scripts_urls[script_init_name]


    def fetch_script(self, script_name):
        url = self._return_script_url(script_name)
        print(f"Fetching script from: {url}")
        result = subprocess.run(["wget", "-qO-", url], capture_output=True, text=True)
        return result.stdout

    def run_script(self, script):
        subprocess.run(["bash", "-"], input=script, text=True)


    def _check_gwx_mode(self):
        if self.mode == "exit-gateway":
            return True
        else:
            return False

    def run_nym_node(self):
        print(f"Running nym-node in mode: {self.mode}")
        subprocess.run([
            os.path.expanduser("~/nym-binaries/nym-node"),
            "run",
            "--mode", self.mode
        ])


if __name__ == '__main__':
    cli = NodeSetupCLI()
    #cli.run_script(cli.prereqs_install_sh)
    #cli.run_script(cli.env_vars_install_sh)
    #cli.run_script(cli.node_install_sh)
    #cli.run_script(cli.service_config_sh)
    #cli._check_gwx_mode() and cli.run_script(cli.landing_page_html)
    #cli._check_gwx_mode() and cli.run_script(cli.nginx_proxy_wss_sh)
