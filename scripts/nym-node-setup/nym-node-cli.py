#!/usr/bin/python3

import os
import subprocess
import tempfile
import shlex
from pathlib import Path
from typing import Iterable, Optional, Mapping

class NodeSetupCLI:

    def __init__(self):
        self.branch = "feature/node-setup-cli"
        self.welcome_message = self.print_welcome_message()
        self.mode = self.prompt_mode()
        self.prereqs_install_sh = self.fetch_script("nym-node-prereqs-install.sh")
        self.env_vars_install_sh = self.fetch_script("setup-env-vars.sh")
        self.node_install_sh = self.fetch_script("nym-node-install.sh")
        self.service_config_sh = self.fetch_script("setup-systemd-service-file.sh")
        self.landing_page_html = self._check_gwx_mode() and self.fetch_script("landing-page.html")
        self.nginx_proxy_wss_sh = self._check_gwx_mode() and self.fetch_script("nginx_proxy_wss_sh")
        self.tunnel_manager_sh = self._check_gwx_mode() and self.fetch_script("network_tunnel_manager.sh")
        self.wg_ip_tables_manager_sh = self._check_gwx_mode() and self.fetch_script("wireguard-exit-policy-manager.sh")
        self.wg_ip_tables_test_sh = self._check_gwx_mode() and self.fetch_script("exit-policy-tests.sh")


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

    def prompt_mode(self):
        mode = input("\
                \nEnter the mode you want to run nym-node in: \
                \n1) mixnode \
                \n2) entry-gateway \
                \n3) exit-gateway \
                \nPress 1, 2 or 3 and enter: \
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

    def fetch_script(self, script_name):
        url = self._return_script_url(script_name)
        print(f"Fetching file from: {url}")
        result = subprocess.run(["wget", "-qO-", url], capture_output=True, text=True)
        if result.returncode != 0 or not result.stdout.strip():
            print(f"wget failed to download the file.")
            print("stderr:", result.stderr)
            raise RuntimeError(f"Failed to fetch {url}")
        # Optional sanity check:
        first_line = result.stdout.splitlines()[0] if result.stdout else ""
        print(f"Downloaded {len(result.stdout)} bytes.")
        return result.stdout

    def _return_script_url(self, script_init_name):
        github_raw_nymtech_nym_scripts_url = f"https://raw.githubusercontent.com/nymtech/nym/refs/heads/{self.branch}/scripts/"
        scripts_urls = {
                "nym-node-prereqs-install.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/nym-node-prereqs-install.sh",
                "setup-env-vars.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/setup-env-vars.sh",
                "nym-node-install.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/nym-node-install.sh",
                "setup-systemd-service-file.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/setup-systemd-service-file.sh",
                "nginx_proxy_wss_sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/setup-nginx-proxy-wss.sh",
                "landing-page.html": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/landing-page.html",
                "network_tunnel_manager.sh": f"{github_raw_nymtech_nym_scripts_url}network_tunnel_manager.sh",
                "wireguard-exit-policy-manager.sh": f"{github_raw_nymtech_nym_scripts_url}wireguard-exit-policy/wireguard-exit-policy-manager.sh",
                "exit-policy-tests.sh": f"{github_raw_nymtech_nym_scripts_url}wireguard-exit-policy/exit-policy-tests.sh",
                }
        return scripts_urls[script_init_name]

    def run_script(
        self,
        script_text: str,
        args: Optional[Iterable[str]] = None,
        env: Optional[Mapping[str, str]] = None,
        cwd: Optional[str] = None,
        sudo: bool = False,
    ) -> int:
        """Save script to a temp file and run it interactively. Returns exit code."""
        path = self._write_temp_script(script_text)
        try:
            cmd = ([ "sudo", str(path) ] if sudo else [ str(path) ]) + (list(args) if args else [])
            run_env = {**os.environ, **(env or {})}
            cp = subprocess.run(cmd, env=run_env, cwd=cwd)
            #if cp.returncode != 0:
            #    raise RuntimeError(f"Script failed with exit code {cp.returncode}")
            #return cp.returncode
        finally:
            try:
                path.unlink(missing_ok=True)
            except Exception:
                pass

    def _write_temp_script(self, script_text: str) -> Path:
       """Write script text to a temp file, ensure bash shebang, chmod +x, return its Path."""
       if not script_text.lstrip().startswith("#!"):
           script_text = "#!/usr/bin/env bash\n" + script_text
       with tempfile.NamedTemporaryFile("w", delete=False, suffix=".sh") as f:
           f.write(script_text)
           path = Path(f.name)
       os.chmod(path, 0o700)  # executable for owner
       return path

    def _check_gwx_mode(self):
        if self.mode == "exit-gateway":
            return True
        else:
            return False

    def check_wg_enabled(self):
        wireguard = os.environ.get("WIREGUARD")
        while wireguard is None:
            user_input = input(
                "Wireguard is not configured.\n"
                "Please note that a node routing WireGuard will be listed as both entry and exit in the application."
                "Enable Wireguard support? (y/n): "
            ).strip().lower()

            if user_input == "y":
                os.environ["WIREGUARD"] = "true"
                wireguard = True
            elif user_input == "n":
                os.environ["WIREGUARD"] = "false"
                wireguard = False
            else:
                print("Invalid input. Please press 'y' or 'n' and press enter.")
        return wireguard

    def run_bash_command(self, command, args=None):
        args = args or []

        if os.path.exists(os.path.expanduser(source)):
            # It's a file path
            path = os.path.expanduser(source)
            print(f"Running script at path: {path} with args: {args}")
            subprocess.run([path] + args)
        else:
            # Treat as raw script content
            print(f"Running inline script with args: {args}")
            subprocess.run(["bash", "-s"] + args, input=source, text=True)

    def run_tunnel_manager_setup(self):
        print(
            "Setting up network configuration for mixnet IP router and Wireguard tunneling. More info: https://nym.com/docs/operators/nodes/nym-node/configuration#1-download-network_tunnel_managersh-make-executable-and-run"
            "This may take a while, follow the steps below and don't kill the process..."
            )
        args = [
            " ",
            "check_nymtun_iptables",
            "remove_duplicate_rules nymtun0",
            "remove_duplicate_rules nymwg",
            "check_nymtun_iptables",
            "adjust_ip_forwarding",
            "apply_iptables_rules",
            "check_nymtun_iptables",
            "apply_iptables_rules_wg",
            "configure_dns_and_icmp_wg",
            "adjust_ip_forwarding",
            "check_ipv6_ipv4_forwarding",
            "joke_through_the_mixnet",
            "joke_through_wg_tunnel",
            ]
        for arg in args:
            parsed_args = shlex.split(arg)
            self.run_bash_command(self.tunnel_manager_sh, parsed_Args)

    def setup_test_wg_ip_tables(self):
        print(
            "Setting up Wireguard IP tables to match Nym exit policy for mixnet, stored at: https://nymtech.net/.wellknown/network-requester/exit-policy.txt"
            "This may take a while, follow the steps below and don't kill the process..."
            )
        self.run_bash_command(self.wg_ip_tables_manager_sh,  ["install"])
        self.run_bash_command(self.wg_ip_tables_manager_sh,  ["status"])
        self.run_bash_command(self.wg_ip_tables_test_sh)

    def run_nym_node_as_service(self):
        service_path = "/etc/systemd/system/nym-node.service"
        print(
            "We are going to start nym-node.service from systemd config located at: /etc/systemd/system/nym-node.service"
            )
        if not os.path.isfile(service_path):
            print(f"Service file not found at {service_path}. Generating one now...")
            self.run_script(self.service_config_sh)
        else:
            print(f"Service file found at {service_path}")

        while True:
            prompt = input("Do you want to start the service now? [y/n]: ").strip().lower()
            if prompt == 'y':
                command = ["service", "nym-node", "start"]
                self.run_bash_command(command)
                print(
                    "nym-node.service started, you can check status or live journal with these commands:\n"
                    "`service nym-node status`\n"
                    "`journalctl -u nym-node -f --all`"
                  )
                break
            elif prompt == 'n':
                print(
                    "Nym node service has not been started. Make sure to run it your nym-node.service before bonding!\n"
                    "You can do it manually:\n"
                    "`service nym-node start`"
                )
                break
            else:
                print("Invalid input. Please press 'y' or 'n' and press enter.")

    def run_bonding_prompt(self):
        print("Time to bond your node to Nyx account, to register it to Nym network")
        node_path = "$HOME/nym-binaries/nym-node"
        if not os.path.isfile(node_path):
            print(f"Nym node not found at {node_path}, we cannot run a bonding prompt!")
            exit(1)
        else:
            while True:
                subprocess.run([
                os.path.expanduser(node_path),
                "bonding-information",
            ])
                self.bash_run_command("curl", ["-4", '"https://ifconfig.me"']),
                print(
                  "====================================\n"
                  "FOLLOW THESE STEPS TO BOND YOUR NODE\n"
                  "====================================\n"
                  "- Open your wallet and go to Bonding menu\n"
                  "- Fill your IP address (printed above) to the Host field\n"
                  "- Setup your operators cost and profit margin\n"
                  "- Copy the long contract message from your wallet\n"
                  )
                msg = "- Paste the contract message from clipboard here and press enter:\n"
                contract_msg = input(msg).strip()
                subprocess.run([
                os.path.expanduser(node_path),
                "sign",
                "--contract-msg",
                contract_msg
            ])
                print(
                  "- Copy the last last part of the string back to your Nym wallet\n"
                  "- Confirm the transaction"
                  )
                confirmation = input(
                  "Did it work out?"
                  "1. YES"
                  "2. NO, try again"
                  "3. Skip for now"
                  "Press 1, 2, or 3 and enter:\n"
                  )
                if confirmation == "1":
                    print("Congratulation, your Nym node is registered to Nym network, wait until the end of epoch for the change to propagate (max 60 min)")
                    break
                elif confirmation == "3":
                    print(
                      "Your node is not bonded, we are skipping this step.\n"
                      "Note that without bonding network tunnel manager will not work fully!\n"
                      "You can always bond manually using:\n"
                      "`$HOME/nym-binaries/nym-node sign --contract-msg <CONTRACT_MESSAGE>`"
                      )
                    break
                elif confirmation == "2":
                    continue
                else:
                    print(
                      "Your input was wrong, we are skipping this step. You can always bond manually using:\n"
                      "`$HOME/nym-binaries/nym-node sign --contract-msg <CONTRACT_MESSAGE>`"
                      )
                    break


if __name__ == '__main__':
    cli = NodeSetupCLI()
    cli.run_script(cli.prereqs_install_sh)
    cli.run_script(cli.env_vars_install_sh)
    cli.run_script(cli.node_install_sh)
    cli.run_script(cli.service_config_sh)
    cli._check_gwx_mode() and cli.run_script(cli.nginx_proxy_wss_sh)
    cli.run_nym_node_as_service()
    cli.run_bonding_prompt()
    cli._check_gwx_mode() and cli.run_script(cli.run_tunnel_manager_setup)
    cli.check_wg_enabled() and cli.run_script(cli.setup_test_wg_ip_tables)
