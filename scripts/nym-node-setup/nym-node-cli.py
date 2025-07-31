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

    def print_welcome_message(self):
        msg = "Welcome to NymNodeCLI, an interactive tool to download, install, setup and run nym-node."
        print(msg)

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
                }
        return scripts_urls[script_init_name]


    def fetch_script(self, script_name):
        url = self._return_script_url(script_name)
        print(f"Fetching script from: {url}")
        result = subprocess.run(["wget", "-qO-", url], capture_output=True, text=True)
        return result.stdout

    def run_script(self, script):
        subprocess.run(["bash", "-"], input=script, text=True)

    def run_nym_node(self):
        print(f"Running nym-node in mode: {self.mode}")
        subprocess.run([
            os.path.expanduser("~/nym-binaries/nym-node"),
            "run",
            "--mode", self.mode
        ])


if __name__ == '__main__':
    cli = NodeSetupCLI()
