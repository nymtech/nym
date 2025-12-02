#!/usr/bin/python3

__version__ = "1.2.0"
__default_branch__ = "develop"

import os
import re
import sys
import subprocess
import argparse
import tempfile
import shlex
import time
from datetime import datetime
from pathlib import Path
from typing import Iterable, Optional, Mapping
from typing import Optional, Tuple

class NodeSetupCLI:
    """All CLI main functions"""

    def __init__(self, args):
        self.branch = args.dev
        self.welcome_message = self.print_welcome_message()
        self.mode = self._get_or_prompt_mode(args)
        self.prereqs_install_sh = self.fetch_script("nym-node-prereqs-install.sh")
        self.node_install_sh = self.fetch_script("nym-node-install.sh")
        self.service_config_sh = self.fetch_script("setup-systemd-service-file.sh")
        self.start_node_systemd_service_sh = self.fetch_script("start-node-systemd-service.sh")
        self.is_gwx = self.mode == "exit-gateway"
        if self.is_gwx:
            self.landing_page_html = self.fetch_script("landing-page.html")
            self.nginx_proxy_wss_sh = self.fetch_script("nginx_proxy_wss_sh")
            self.tunnel_manager_sh = self.fetch_script("network_tunnel_manager.sh")
            self.quic_bridge_deployment_sh = self.fetch_script("quic_bridge_deployment.sh")
        else:
            self.landing_page_html = None
            self.nginx_proxy_wss_sh = None
            self.tunnel_manager_sh = None
            self.wg_ip_tables_manager_sh = None
            self.wg_ip_tables_test_sh = None
            self.quic_bridge_deployment_sh = None


    def print_welcome_message(self):
        """Welcome user, warns for needed pre-reqs and asks for confimation"""
        self.print_character("=", 41)
        print(\
            "* * * * * * NYM - NODE - CLI * * * * * *\n" \
            "An interactive tool to download, install\n" \
            "* * * * * setup & run nym-node * * * * *"
            )
        self.print_character("=", 41)
        msg = \
            "Before you begin, make sure that:\n"\
            "1. You run this setup on Debian based Linux (ie Ubuntu 22.04 LTS)\n"\
            "2. You run this installation program from a root shell\n"\
            "3. You meet minimal requirements: https://nym.com/docs/operators/nodes\n"\
            "4. You accept Operators Terms & Conditions: https://nym.com/operators-validators-terms\n"\
            "5. You have Nym wallet with at least 101 NYM: https://nym.com/docs/operators/nodes/preliminary-steps/wallet-preparation\n"\
            "6. In case of Gateway behind reverse proxy, you have A and AAAA DNS record pointing to this IP and propagated\n"\
            "\nTo confirm and continue, write 'ACCEPT' and press enter:"
        print(msg)
        confirmation = input("\n")
        if confirmation.upper() == "ACCEPT":
            pass
        else:
            print("Without confirming the points above, we cannot continue.")
            exit(1)
    
    def ensure_env_values(self, args):
        """Collect env vars from args or prompt interactively, then save to env.sh."""
        env_file = Path("env.sh")
        fields = [
            ("hostname", "HOSTNAME", "Enter hostname (if you don't use a DNS, press enter): "),
            ("location", "LOCATION", "Enter node location (country code or name): "),
            ("email", "EMAIL", "Enter your email: "),
            ("moniker", "MONIKER", "Enter node public moniker (visible in explorer & NymVPN app): "),
            ("description", "DESCRIPTION", "Enter short node public description: "),
        ]

        existing = self._read_env_file(env_file)
        updated = {}

        for arg_name, key, prompt in fields:
            cli_val = getattr(args, arg_name, None)
            value = cli_val.strip() if cli_val else existing.get(key) or input(prompt).strip()
            updated[key] = value
            os.environ[key] = value

        # autodetect PUBLIC_IP if not already set
        if not os.environ.get("PUBLIC_IP"):
            try:
                ip = subprocess.run(["curl", "-fsS4", "https://ifconfig.me"],
                                    capture_output=True, text=True, timeout=5)
                if ip.returncode == 0 and ip.stdout.strip():
                    updated["PUBLIC_IP"] = ip.stdout.strip()
                    os.environ["PUBLIC_IP"] = ip.stdout.strip()
            except subprocess.TimeoutExpired:
                print("[WARN] Timeout expired while trying to fetch public IP with curl.")
            except FileNotFoundError:
                print("[WARN] 'curl' command not found. Please install curl or set PUBLIC_IP manually.")
            except subprocess.CalledProcessError as e:
                print(f"[WARN] Error while running curl to fetch public IP: {e}")

        # write all collected variables to env.sh in one go
        self._upsert_env_vars(updated, env_file)

        print(f"[OK] Updated env.sh with {len(updated)} entries.")




    def _upsert_env_vars(self, updates: dict, env_file: Path = Path("env.sh")):
        existing = self._read_env_file(env_file)
        existing.update(updates)
        with env_file.open("w") as f:
            for k, v in existing.items():
                f.write(f'export {k}="{v}"\n')
        os.environ.update(updates)

    def _read_env_file(self, env_file: Path) -> dict:
        env = {}
        if env_file.exists():
            for line in env_file.read_text().splitlines():
                if line.startswith("export ") and "=" in line:
                    k, v = line.replace("export ", "", 1).split("=", 1)
                    env[k.strip()] = v.strip().strip('"')
        return env
    
    def _get_or_prompt_mode(self, args):
        """Resolve MODE from --mode, env.sh, os.environ, or prompt; persist to env.sh."""

        env_file = Path("env.sh")

        # CLI arg
        mode = getattr(args, "mode", None)
        if mode:
            mode = mode.strip().lower()
            self._upsert_env_vars({"MODE": mode})
            print(f"Mode set to '{mode}' from CLI argument.")
            return mode

        # env.sh (replaces manual read)
        existing = self._read_env_file(env_file)
        mode = existing.get("MODE")
        if mode:
            os.environ["MODE"] = mode
            return mode

        # process env
        if os.environ.get("MODE"):
            return os.environ["MODE"]

        # prompt
        mode = input(
            "\nEnter node mode (mixnode / entry-gateway / exit-gateway): "
        ).strip().lower()
        if mode not in ("mixnode", "entry-gateway", "exit-gateway"):
            print("Invalid mode. Must be one of: mixnode, entry-gateway, exit-gateway.")
            raise SystemExit(1)

        self._upsert_env_vars({"MODE": mode})
        print(f"Mode set to '{mode}' — stored in env.sh and sourced for immediate use.")
        return mode

    def fetch_script(self, script_name):
        """Fetches needed scripts according to a defined mode"""
        # print header only the first time
        if not getattr(self, "_fetched_once", False):
            print("\n* * * Fetching required scripts * * *")
            self._fetched_once = True
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
        """Dictionary pointing to scripts url returning value according to a passed key"""
        github_raw_nymtech_nym_scripts_url = f"https://raw.githubusercontent.com/nymtech/nym/refs/heads/{self.branch}/scripts/"
        scripts_urls = {
                "nym-node-prereqs-install.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/nym-node-prereqs-install.sh",
                "nym-node-install.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/nym-node-install.sh",
                "setup-systemd-service-file.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/setup-systemd-service-file.sh",
                "start-node-systemd-service.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/start-node-systemd-service.sh",
                "nginx_proxy_wss_sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/setup-nginx-proxy-wss.sh",
                "landing-page.html": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/landing-page.html",
                "network_tunnel_manager.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/network-tunnel-manager.sh",
                "quic_bridge_deployment.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/quic_bridge_deployment.sh"
                }

        return scripts_urls[script_init_name]

    def run_script(
        self,
        script_text: str,
        args: Optional[Iterable[str]] = None,
        env: Optional[Mapping[str, str]] = None,
        cwd: Optional[str] = None,
        sudo: bool = False,         # ignored for root; kept for signature compat
        detached: bool = False,
    ) -> int:
        """
        Save script to a temp file and run it
        - Automatically injects ENV_FILE=<abs path to ./env.sh> unless already provided
        - Adds SYSTEMD_PAGER="" and SYSTEMD_COLORS="0" by default
        Returns exit code (0 if detached fire-and-forget)
        """
        import os, subprocess

        path = self._write_temp_script(script_text)
        try:
            # build env with sensible defaults
            run_env = dict(os.environ)
            if env:
                run_env.update(env)

            # ensure ENV_FILE is absolute and present for all scripts
            if "ENV_FILE" not in run_env:
                # if env.sh is elsewhere, change this to your known base dir
                env_file = os.path.abspath(os.path.join(os.getcwd(), "env.sh"))
                run_env["ENV_FILE"] = env_file

            # make systemctl non-interactive everywhere
            run_env.setdefault("SYSTEMD_PAGER", "")
            run_env.setdefault("SYSTEMD_COLORS", "0")

            cmd = [str(path)] + (list(args) if args else [])

            if detached:
                subprocess.Popen(
                    cmd,
                    env=run_env,
                    cwd=cwd,
                    stdin=subprocess.DEVNULL,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    start_new_session=True,
                    close_fds=True,
                )
                return 0
            else:
                cp = subprocess.run(cmd, env=run_env, cwd=cwd)
                return cp.returncode
        finally:
            try:
                path.unlink(missing_ok=True)
            except Exception:
                pass

    def _write_temp_script(self, script_text: str) -> Path:
        """Helper: write script text to a temp file, ensure bash shebang, chmod +x, return its path"""
        if not script_text.lstrip().startswith("#!"):
            script_text = "#!/usr/bin/env bash\n" + script_text
        with tempfile.NamedTemporaryFile("w", delete=False, suffix=".sh") as f:
            f.write(script_text)
            path = Path(f.name)
        os.chmod(path, 0o700)
        return path

    def _check_gwx_mode(self):
        """Helper: Several fns run only for GWx - this fn checks this condition"""
        return self.mode == "exit-gateway"

    def check_wg_enabled(self, args=None):
        """Determine if WireGuard is enabled; precedence: CLI > env > env.sh > prompt. Persist normalized value."""

        env_file = os.path.join(os.getcwd(), "env.sh")

        def norm(v):
            return "true" if str(v).strip().lower() == "true" else "false"

        val = None

        # CLI argument
        if args and getattr(args, "wireguard", None) is not None:
            val = norm(getattr(args, "wireguard"))
            print(f"[INFO] WireGuard mode provided via CLI: {val}")

        # Environment variable
        val = val or os.environ.get("WIREGUARD")

        # env.sh file
        if val is None:
            envs = self._read_env_file(Path(env_file))
            val = envs.get("WIREGUARD")

        # Prompt
        if val is None:
            ans = input(
                "\nWireGuard is not configured.\n"
                "Nodes routing WireGuard can be listed as both entry and exit in the app.\n"
                "Enable WireGuard support? (Y/n): "
            ).strip().lower()
            val = "true" if ans in ("", "y", "yes") else "false"

        val = norm(val)
        os.environ["WIREGUARD"] = val

        # Persist to env.sh
        try:
            text = ""
            if os.path.isfile(env_file):
                with open(env_file, encoding="utf-8") as f:
                    text = f.read()
            if re.search(r'^\s*export\s+WIREGUARD\s*=.*$', text, re.M):
                text = re.sub(r'^\s*export\s+WIREGUARD\s*=.*$', f'export WIREGUARD="{val}"', text, flags=re.M)
            else:
                text = (text.rstrip("\n") + "\n" if text else "") + f'export WIREGUARD="{val}"\n'
            with open(env_file, "w", encoding="utf-8") as f:
                f.write(text)
            print(f'WIREGUARD={val} saved to {env_file}')
        except OSError as e:
            print(f"Warning: could not write {env_file}: {e}")

        return val == "true"


    def run_bash_command(self, command, args=None, *, env=None, cwd=None, check=True):
        """
        Run a command with optional args (no script stdin)
        `command` can be a string (e.g., "ls") or a list (e.g., ["ls", "-la"]).
        """
        # normalize command into a list
        if isinstance(command, str):
            cmd = shlex.split(command)
        else:
            cmd = list(command)

        if args:
            cmd += list(args)

        print("Running:", " ".join(shlex.quote(c) for c in cmd))
        return subprocess.run(cmd, env=env, cwd=cwd, check=check)


    def run_tunnel_manager_setup(self):
        """A standalone fn to pass full cmd list needed for correct setup and test network tunneling, using an external script"""
        print(
            "\n* * * Setting up network configuration for mixnet IP router and Wireguard tunneling * * *"
            "\nMore info: https://nym.com/docs/operators/nodes/nym-node/configuration#routing-configuration"
            "\nThis may take a while; follow the steps below and don't kill the process..."
        )

        # each entry is the exact argv to pass to the script
        steps = [
            ["complete_networking_configuration"]
        ]

        for argv in steps:
            print("Running: network_tunnel_manager.sh", *argv)
            rc = self.run_script(self.tunnel_manager_sh, args=argv)
            if rc != 0:
                print(f"Step {' '.join(argv)} failed with exit code {rc}. Stopping.")
                return rc

        print("Network tunnel manager setup completed successfully.")
        return 0

    def setup_test_wg_ip_tables(self):
        """Configuration and test of Wireguard exit policy according to mixnet exit policy using external scripts"""
        print(
            "Setting up Wireguard IP tables to match Nym exit policy for mixnet, stored at: https://nymtech.net/.wellknown/network-requester/exit-policy.txt"
            "\nThis may take a while, follow the steps below and don't kill the process..."
            )
        self.run_script(self.tunnel_manager_sh,  args=["exit_policy_install"])

    def quic_bridge_deploy(self):
        """Setup QUIC bridge and configuration using external script"""
        print("\n* * * Installing and configuring QUIC bridges * * *")
        answer = input("\nDo you want to install, setup and run QUIC bridge? (Y/n) ").strip().lower()

        if answer in ("", "y", "yes"):
            self.run_script(self.quic_bridge_deployment_sh, args=["full_bridge_setup"])
        else:
            print("Skipping QUIC bridge setup.")    

    def run_nym_node_as_service(self):
        """Starts /etc/systemd/system/nym-node.service based on prompt using external script"""
        service = "nym-node.service"
        service_path = "/etc/systemd/system/nym-node.service"
        print(f"\n* * * We are going to start {service} from systemd config located at: {service_path} * * *")

        # if the service file is missing, run setup non-interactively
        if not os.path.isfile(service_path):
            print(f"Service file not found at {service_path}. Running setup...")
            setup_env = {
                **os.environ,
                "SYSTEMD_PAGER": "",
                "SYSTEMD_COLORS": "0",
                "NONINTERACTIVE": "1",
                "MODE": os.environ.get("MODE", "mixnode"),
            }
            self.run_script(self.service_config_sh, env=setup_env)
            if not os.path.isfile(service_path):
                print("Service file still not found after setup. Aborting.")
                return

        run_env = {**os.environ, "SYSTEMD_PAGER": "", "SYSTEMD_COLORS": "0", "WAIT_TIMEOUT": "600"}
        is_active = subprocess.run(["systemctl", "is-active", "--quiet", service], env=run_env).returncode == 0

        if is_active:
            while True:
                ans = input(f"{service} is already running. Restart it now? (Y/n):\n").strip().lower()
                if ans in ("", "Y", "y"):
                    self.run_script(self.start_node_systemd_service_sh, args=["restart-poll"], env=run_env)
                    return
                elif ans == "n":
                    print("Continuing without restart.")
                    return
                else:
                    print("Invalid input. Please press 'y' or 'n' and press enter.")
        else:
            while True:
                ans = input(f"{service} is not running. Start it now? (Y/n):\n").strip().lower()
                if ans in ("", "Y", "y"):
                    self.run_script(self.start_node_systemd_service_sh, args=["start-poll"], env=run_env)
                    return
                elif ans == "n":
                    print("Okay, not starting it.")
                    return
                else:
                    print("Invalid input. Please press 'y' or 'n' and press enter.")



    def run_bonding_prompt(self):
        """Interactive function navigating user to bond node"""
        print("\n")
        print("* * * Bonding Nym Node * * *")
        print("Time to register your node to Nym Network by bonding it using Nym wallet ...")
        node_path = os.path.expandvars(os.path.expanduser("$HOME/nym-binaries/nym-node"))
        if not (os.path.isfile(node_path) and os.access(node_path, os.X_OK)):
            print(f"Nym node not found at {node_path}, we cannot run a bonding prompt!")
            exit(1)
        else:
            while True:
                subprocess.run([os.path.expanduser(node_path), "bonding-information"])
                self.run_bash_command(command="curl", args=["-4", "https://ifconfig.me"])
                print("\n")
                self.print_character("=", 56)
                print("* * *  FOLLOW  THESE  STEPS  TO  BOND  YOUR  NODE  * * *")
                print("If you already bonded your node before, just press enter")
                self.print_character("=", 56)
                print(
                  "1. Open your wallet and go to Bonding menu\n"
                  "2. Paste Identity key and your IP address (printed above)\n"
                  "3. Setup your operators cost and profit margin\n"
                  "4. Copy the long contract message from your wallet"
                )
                msg = "5. Paste the contract message from clipboard here and press enter:\n"
                contract_msg = input(msg).strip()
                if contract_msg == "":
                    print("Skipping bonding process as your node is already bonded\n")
                    return
                else:
                    subprocess.run([
                        os.path.expanduser(node_path),
                        "sign",
                        "--contract-msg",
                        contract_msg
                    ])
                    print(
                      "6. Copy the last part of the string back to your Nym wallet\n"
                      "7. Confirm the transaction"
                    )
                confirmation = input(
                  "\n* * * Is your node bonded?\n"
                  "1. YES\n"
                  "2. NO, try again\n"
                  "3. Skip bonding for now\n"
                  "Press 1, 2, or 3 and enter:\n"
                ).strip()

                if confirmation == "1":
                    # NEW: fetch identity + composed message and print it
                    _, message = self._explorer_message_from_identity(node_path)
                    self.print_character("*", 42)
                    print(message)
                    self.print_character("*", 42)
                    return
                elif confirmation == "3":
                    print(
                      "Your node is not bonded, we are skipping this step.\n"
                      "Note that without bonding network tunnel manager will not work fully!\n"
                      "You can always bond manually using:\n"
                      "`$HOME/nym-binaries/nym-node sign --contract-msg <CONTRACT_MESSAGE>`"
                    )
                    return
                elif confirmation == "2":
                    continue
                else:
                    print(
                      "Your input was wrong, we are skipping this step. You can always bond manually using:\n"
                      "`$HOME/nym-binaries/nym-node sign --contract-msg <CONTRACT_MESSAGE>`"
                    )
                    return

    def _explorer_message_from_identity(self, node_path: str) -> Tuple[Optional[str], str]:
        """
        Runs `$HOME/nym-binaries/nym-node bonding-information` to
        extract the id_key and returns explorer URL with a message
        else return the message without the URL
        """
        try:
            cp = subprocess.run(
                [os.path.expanduser(node_path), "bonding-information"],
                capture_output=True, text=True, check=False, timeout=30
            )
            output = cp.stdout or ""
        except Exception as e:
            output = ""
            # still return the generic message
            key = None
            msg = (
                "* * * C O N G R A T U L A T I O N ! * * *\n"
                "Your Nym node is registered to Nym network\n"
                "Wait until the end of epoch for the change\n"
                "to propagate (max 60 min)\n"
                "(Could not obtain Identity Key automatically.)"
            )
            return key, msg

        # parse the id_key
        m = re.search(r"^Identity Key:\s*([A-Za-z0-9]+)\s*$", output, flags=re.MULTILINE)
        key = m.group(1) if m else None

        base_msg = (
            "* * * C O N G R A T U L A T I O N ! * * *\n"
            "Your Nym node is registered to Nym network\n"
            "Wait until the end of epoch for the change\n"
            "to propagate (max 60 min)\n"
        )

        if key:
            url = f"https://explorer.nym.spectredao.net/nodes/{key}"
            msg = base_msg + f"Then you can see your node at:\n{url}"
        else:
            msg = base_msg + "(Could not obtain Identity Key automatically.)"

        return key, msg

    def print_character(self, ch: str, count: int):
        """Print `ch` repeated `count` times (no unbounded growth)"""
        if not ch:
            return
        # Use exactly one codepoint char; trim if longer
        ch = ch[:1]
        # Clamp count to a sensible max to avoid huge outputs
        try:
            n = int(count)
        except Exception:
            n = 0
        n = max(0, min(n, 161))
        print(ch * n)

    def _env_with_envfile(self) -> dict:
        """Helper for env persistence sanity"""
        env = dict(os.environ)
        env["SYSTEMD_PAGER"] = ""
        env["SYSTEMD_COLORS"] = "0"
        env["ENV_FILE"] = os.path.abspath(os.path.join(os.getcwd(), "env.sh"))
        return env

    def run_node_installation(self,args):
        """Main function called by argparser command install running full node install flow"""
        self.ensure_env_values(args)
        # Pass uplink override to all helper scripts if provided
        if getattr(args, "uplink_dev_v4", None):
            os.environ["IPV4_UPLINK_DEV"] = args.uplink_dev_v4
        if getattr(args, "uplink_dev_v6", None):
            os.environ["IPV6_UPLINK_DEV"] = args.uplink_dev_v6
        self.run_script(self.prereqs_install_sh)
        self.run_script(self.node_install_sh)
        self.run_script(self.service_config_sh)
        self._check_gwx_mode() and self.run_script(self.nginx_proxy_wss_sh)
        self.run_nym_node_as_service()
        self.run_bonding_prompt()
        if self._check_gwx_mode():
            self.run_tunnel_manager_setup()
            if self.check_wg_enabled():
                self.setup_test_wg_ip_tables()
                self.quic_bridge_deploy()



class ArgParser:
    """CLI argument interface managing the NodeSetupCLI functions based on user input"""

    def parser_main(self):
        # shared options to work before and after subcommands
        parent = argparse.ArgumentParser(add_help=False)
        parent.add_argument(
            "-V", "--version",
            action="version",
            version=f"nym-node-cli {__version__}"
        )
        parent.add_argument("-d", "--dev", metavar="BRANCH",
                            help="Define github branch (default: develop)",
                            type=str,
                            default=argparse.SUPPRESS)
        parent.add_argument("-v", "--verbose", action="store_true",
                            help="Show full error tracebacks")

        parser = argparse.ArgumentParser(
            prog="nym-node-cli",
            description="An interactive tool to download, install, setup and run nym-node",
            epilog="Privacy infrastructure operated by people around the world",
            parents=[parent],
        )

        subparsers = parser.add_subparsers(dest="command", help="subcommands")
        subparsers.required = True

        install_parser = subparsers.add_parser(
            "install", parents=[parent],
            help="Starts nym-node installation setup CLI",
            aliases=["i", "I"], add_help=True
        )
        install_parser.add_argument(
            "--mode",
            choices=["mixnode", "entry-gateway", "exit-gateway"],
            help="Node mode: 'mixnode', 'entry-gateway', or 'exit-gateway'",
        )
        install_parser.add_argument(
            "--wireguard-enabled",
            choices=["true", "false"],
            help="WireGuard functionality switch: true / false"
        )
        install_parser.add_argument("--hostname", help="Node domain / hostname")
        install_parser.add_argument("--location", help="Node location (country code or name)")
        install_parser.add_argument("--email", help="Contact email for the node operator")
        install_parser.add_argument("--moniker", help="Public moniker displayed in explorer & NymVPN app")
        install_parser.add_argument("--description", help="Short public description of the node")
        install_parser.add_argument("--public-ip", help="External IPv4 address (autodetected if omitted)")
        install_parser.add_argument("--nym-node-binary", help="URL for nym-node binary (autodetected if omitted)")
        install_parser.add_argument("--uplink-dev-v4", help="Override ipv4 uplink interface used for NAT/FORWARD (e.g., 'eth0'; autodetected if omitted)")
        install_parser.add_argument("--uplink-dev-v6", help="Override ipv6 uplink interface used for NAT/FORWARD (e.g., 'eth0.1'; autodetected if omitted)")
        
        # generic fallback
        install_parser.add_argument(
            "--env",
            action="append",
            metavar="KEY=VALUE",
            help="(Optional) Extra ENV VARS, e.g. --env CUSTOM_KEY=value",
        )


        args = parser.parse_args()

        # assign default manually only if user didn’t supply --dev
        if not hasattr(args, "dev"):
            args.dev = __default_branch__

        try:
            # build CLI with parsed args to catch errors soon
            cli = NodeSetupCLI(args)

            commands = {
                "install": cli.run_node_installation,
                "i":       cli.run_node_installation,
                "I":       cli.run_node_installation,
            }

            func = commands.get(args.command)
            if func is None:
                parser.print_help()
                parser.error(f"Unknown command: {args.command}")

            # execute subcommand within error test
            func(args)

        except SystemExit:
            raise
        except RuntimeError as e:
            print(f"{e}\nMake sure that the your BRANCH ('{args.dev}') provided in --dev option contains this program.")
            sys.exit(1)
        except Exception as e:
            if getattr(args, "verbose", False):
                traceback.print_exc()
            else:
                print(f"error: {e}", file=sys.stderr)
            sys.exit(1)


class SystemSafeGuards:
    """A few safe guards to deal with memory usage by this program"""

    def _protect_from_oom(self, score: int = -900):
        try:
            with open("/proc/self/oom_score_adj", "w") as f:
                f.write(str(score))
        except Exception:
            pass

    def _trim_memory(self):
        """Liberate freeable Python objects and return arenas to the OS if possible"""
        try:
            import gc, ctypes
            gc.collect()
            try:
                libc = ctypes.CDLL("libc.so.6")
                # 0 = “trim as much as possible”
                libc.malloc_trim(0)
            except Exception:
                pass
        except Exception:
            pass

    def _cap_controller_memory(self, bytes_limit: int = 2 * 1024**3):
        # limit this Python process to e.g. 2 GiB virtual memory
        try:
            import resource
            resource.setrlimit(resource.RLIMIT_AS, (bytes_limit, bytes_limit))
        except Exception:
            pass


if __name__ == '__main__':
    safeguards = SystemSafeGuards()
    safeguards._protect_from_oom(-900)             # de-prioritize controller as OOM victim
    safeguards._cap_controller_memory(2 * 1024**3) # optional: cap controller to 2 GiB
    app = ArgParser()
    app.parser_main()
