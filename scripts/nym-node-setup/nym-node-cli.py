#!/usr/bin/python3

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

class NodeSetupCLI:

    def __init__(self, args):
        self.branch = self._branch(args)
        self.welcome_message = self.print_welcome_message()
        self.mode = self.prompt_mode()
        self.prereqs_install_sh = self.fetch_script("nym-node-prereqs-install.sh")
        self.env_vars_install_sh = self.fetch_script("setup-env-vars.sh")
        self.node_install_sh = self.fetch_script("nym-node-install.sh")
        self.service_config_sh = self.fetch_script("setup-systemd-service-file.sh")
        self.start_node_systemd_service_sh = self.fetch_script("start-node-systemd-service.sh")
        self.landing_page_html = self._check_gwx_mode() and self.fetch_script("landing-page.html")
        self.nginx_proxy_wss_sh = self._check_gwx_mode() and self.fetch_script("nginx_proxy_wss_sh")
        self.tunnel_manager_sh = self._check_gwx_mode() and self.fetch_script("network_tunnel_manager.sh")
        self.wg_ip_tables_manager_sh = self._check_gwx_mode() and self.fetch_script("wireguard-exit-policy-manager.sh")
        self.wg_ip_tables_test_sh = self._check_gwx_mode() and self.fetch_script("exit-policy-tests.sh")


    def _branch(self, args):
        branch = args.dev
        return branch

    def print_welcome_message(self):
        self.print_character("=", 41)
        print("* * * * * * NYM - NODE - CLI * * * * * *\nAn interactive tool to download, install\n* * * * * setup & run nym-node * * * * *")
        self.print_character("=", 41)
        msg = \
            "Before you begin, make sure that:\n"\
            "* You run this setup on Debian based Linux (ie Ubuntu)\n"\
            "* You run this installation program from a root shell\n"\
            "* You meet minimal requirements: https://nym.com/docs/operators/nodes\n"\
            "* You agree with Operators Terms & Conditions: https://nym.com/operators-validators-terms\n"\
            "* You have Nym wallet with at least 101 NYM: https://nym.com/docs/operators/nodes/preliminary-steps/wallet-preparation\n"\
            "* In case of Gateway behind reverse proxy, you have A and AAAA DNS record pointing to this IP and propagated\n"\
            "\nTo confirm and continue, write 'YES' and press enter:"
        print(msg)
        confirmation = input("\n")
        if confirmation.upper() == "YES":
            pass
        else:
            print("Without confirming the points above, we cannot continue.")
            exit(1)

    def prompt_mode(self):
        mode = input(
            "\nEnter the mode you want to run nym-node in: "
            "\n1) mixnode "
            "\n2) entry-gateway "
            "\n3) exit-gateway "
            "\nPress 1, 2 or 3 and enter:\n"
        ).strip()

        if mode in ("1", "mixnode"):
            mode = "mixnode"
        elif mode in ("2", "entry-gateway"):
            mode = "entry-gateway"
        elif mode in ("3", "exit-gateway"):
            mode = "exit-gateway"
        else:
            print("Only numbers 1, 2 or 3 are accepted.")
            raise SystemExit(1)

        # Save mode for this Python instance
        self.mode = mode
        os.environ["MODE"] = mode

        # Persist to env.sh so other scripts can source it
        env_file = Path("env.sh")
        with env_file.open("a") as f:
            f.write(f'export MODE="{mode}"\n')

        # Source env.sh so future bash subprocesses see it immediately
        subprocess.run("source ./env.sh", shell=True, executable="/bin/bash")

        print(f"Mode set to '{mode}' — stored in env.sh and sourced for immediate use.")
        return mode


    def fetch_script(self, script_name):
        #print("\n* * * Fetching required scripts * * *")
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
                "start-node-systemd-service.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/start-node-systemd-service.sh",
                "nginx_proxy_wss_sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/setup-nginx-proxy-wss.sh",
                "landing-page.html": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/landing-page.html",
                "network_tunnel_manager.sh": f"https://raw.githubusercontent.com/nymtech/nym/refs/heads/develop/scripts/network_tunnel_manager.sh",
                "wireguard-exit-policy-manager.sh": f"https://raw.githubusercontent.com/nymtech/nym/refs/heads/develop/scripts/wireguard-exit-policy/wireguard-exit-policy-manager.sh",
                "exit-policy-tests.sh": f"https://raw.githubusercontent.com/nymtech/nym/refs/heads/develop/scripts/wireguard-exit-policy/exit-policy-tests.sh",
                }
        return scripts_urls[script_init_name]

    def run_script(
        self,
        script_text: str,
        args: Optional[Iterable[str]] = None,
        env: Optional[Mapping[str, str]] = None,
        cwd: Optional[str] = None,
        sudo: bool = False,         # ignored when you're root; kept for signature compat
        detached: bool = False,
    ) -> int:
        """
        Save script to a temp file and run it.
        - Automatically injects ENV_FILE=<abs path to ./env.sh> unless already provided.
        - Adds SYSTEMD_PAGER="" and SYSTEMD_COLORS="0" by default.
        Returns exit code (0 if detached fire-and-forget).
        """
        import os, subprocess

        path = self._write_temp_script(script_text)
        try:
            # Build env with sensible defaults
            run_env = dict(os.environ)
            if env:
                run_env.update(env)

            # Ensure ENV_FILE is absolute and present for all scripts
            if "ENV_FILE" not in run_env:
                # If you keep env.sh elsewhere, change this to your known base dir
                env_file = os.path.abspath(os.path.join(os.getcwd(), "env.sh"))
                run_env["ENV_FILE"] = env_file

            # Make systemctl non-interactive everywhere
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


        env_file = os.path.abspath(os.path.join(os.getcwd(), "env.sh"))

        def norm(v):  # -> "true" or "false"
            return "true" if str(v).strip().lower() in ("1", "true", "yes", "y") else "false"

        # precedence: process env → env.sh → prompt
        val = os.environ.get("WIREGUARD")

        if val is None and os.path.isfile(env_file):
            try:
                with open(env_file, "r", encoding="utf-8") as f:
                    m = re.search(r'^\s*export\s+WIREGUARD\s*=\s*"?([^"\n]+)"?', f.read(), re.M)
                    if m:
                        val = m.group(1)
            except Exception:
                pass

        if val is None:
            ans = input(
                "\nWireGuard is not configured.\n"
                "Nodes routing WireGuard can be listed as both entry and exit in the app.\n"
                "Enable WireGuard support? (y/n): "
            ).strip().lower()
            val = "true" if ans in ("y", "yes") else "false"

        val = norm(val)
        os.environ["WIREGUARD"] = val

        # persist to env.sh (replace or append)
        try:
            text = ""
            if os.path.isfile(env_file):
                with open(env_file, "r", encoding="utf-8") as f:
                    text = f.read()
            if re.search(r'^\s*export\s+WIREGUARD\s*=.*$', text, re.M):
                text = re.sub(r'^\s*export\s+WIREGUARD\s*=.*$', f'export WIREGUARD="{val}"', text, flags=re.M)
            else:
                if text and not text.endswith("\n"):
                    text += "\n"
                text += f'export WIREGUARD="{val}"\n'
            with open(env_file, "w", encoding="utf-8") as f:
                f.write(text)
            print(f'WIREGUARD={val} saved to {env_file}')
        except Exception as e:
            print(f"Warning: could not write {env_file}: {e}")

        return (val == "true")

    def run_bash_command(self, command, args=None, *, env=None, cwd=None, check=True):
        """
        Run a command with optional args (no script stdin).
        `command` can be a string (e.g., "ls") or a list (e.g., ["ls", "-la"]).
        """
        # Normalize command into a list
        if isinstance(command, str):
            cmd = shlex.split(command)
        else:
            cmd = list(command)

        if args:
            cmd += list(args)

        print("Running:", " ".join(shlex.quote(c) for c in cmd))
        return subprocess.run(cmd, env=env, cwd=cwd, check=check)


    def run_tunnel_manager_setup(self):
        print(
            "\n* * * Setting up network configuration for mixnet IP router and Wireguard tunneling * * *"
            "\nMore info: https://nym.com/docs/operators/nodes/nym-node/configuration#1-download-network_tunnel_managersh-make-executable-and-run"
            "\nThis may take a while; follow the steps below and don't kill the process..."
        )

        # Each entry is the exact argv to pass to the script
        steps = [
            ["check_nymtun_iptables"],
            ["remove_duplicate_rules", "nymtun0"],
            ["remove_duplicate_rules", "nymwg"],
            ["check_nymtun_iptables"],
            ["adjust_ip_forwarding"],
            ["apply_iptables_rules"],          # for nymtun0 inside script
            ["check_nymtun_iptables"],
            ["apply_iptables_rules_wg"],       # for nymwg inside script
            ["configure_dns_and_icmp_wg"],
            ["adjust_ip_forwarding"],
            ["check_ipv6_ipv4_forwarding"],
            ["joke_through_the_mixnet"],
            ["joke_through_wg_tunnel"],
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
        print(
            "Setting up Wireguard IP tables to match Nym exit policy for mixnet, stored at: https://nymtech.net/.wellknown/network-requester/exit-policy.txt"
            "This may take a while, follow the steps below and don't kill the process..."
            )
        self.run_script(self.wg_ip_tables_manager_sh,  args=["install"])
        self.run_script(self.wg_ip_tables_manager_sh,  args=["status"])
        self.run_script(self.wg_ip_tables_test_sh)


    def run_nym_node_as_service(self):
        service = "nym-node.service"
        service_path = "/etc/systemd/system/nym-node.service"
        print(f"We are going to start {service} from systemd config located at: {service_path}")

        # If the service file is missing, run setup non-interactively
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

        # Always run as root, so no sudo needed
        run_env = {**os.environ, "SYSTEMD_PAGER": "", "SYSTEMD_COLORS": "0", "WAIT_TIMEOUT": "600"}
        is_active = subprocess.run(["systemctl", "is-active", "--quiet", service], env=run_env).returncode == 0

        if is_active:
            while True:
                ans = input(f"{service} is already running. Restart it now? [y/n]: ").strip().lower()
                if ans == "y":
                    self.run_script(self.start_node_systemd_service_sh, args=["restart-poll"], env=run_env)
                    return
                elif ans == "n":
                    print("Continuing without restart.")
                    return
                else:
                    print("Invalid input. Please press 'y' or 'n' and press enter.")
        else:
            while True:
                ans = input(f"{service} is not running. Start it now? [y/n]: ").strip().lower()
                if ans == "y":
                    self.run_script(self.start_node_systemd_service_sh, args=["start-poll"], env=run_env)
                    return
                elif ans == "n":
                    print("Okay, not starting it.")
                    return
                else:
                    print("Invalid input. Please press 'y' or 'n' and press enter.")


    def run_bonding_prompt(self):
        print("\n")
        self.print_character("-", 36)
        print("Time to register your node to Nym Network by bonding it using Nym wallet ...")
        node_path = os.path.expandvars(os.path.expanduser("$HOME/nym-binaries/nym-node"))
        # Or: node_path = str(Path.home() / "nym-binaries" / "nym-node")
        if not (os.path.isfile(node_path) and os.access(node_path, os.X_OK)):
            print(f"Nym node not found at {node_path}, we cannot run a bonding prompt!")
            exit(1)
        else:
            while True:
                subprocess.run([
                os.path.expanduser(node_path),
                "bonding-information",
            ])
                self.run_bash_command(command="curl", args=["-4", "https://ifconfig.me"]),
                print("\n")
                self.print_character("=", 36)
                print("FOLLOW THESE STEPS TO BOND YOUR NODE")
                self.print_character("=", 36)
                print(
                  "- Open your wallet and go to Bonding menu\n"
                  "- Paste Identity key and your IP address (printed above)\n"
                  "- Setup your operators cost and profit margin\n"
                  "- Copy the long contract message from your wallet"
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
                  "Did it work out?\n"
                  "1. YES\n"
                  "2. NO, try again\n"
                  "3. Skip for now\n"
                  "Press 1, 2, or 3 and enter:\n"
                  )
                if confirmation == "1":
                    message = \
                        "* * * C O N G R A T U L A T I O N ! * * *\n" \
                        "Your Nym node is registered to Nym network\n" \
                        "Wait until the end of epoch for the change\n" \
                        "to propagate (max 60 min)"
                    self.print_character("*",42)
                    print(message)
                    self.print_character("*",42)
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


    def print_character(self, ch: str, count: int):
        """Print `ch` repeated `count` times (no unbounded growth)."""
        if not ch:
            return
        # Use exactly one codepoint char; trim if longer
        ch = ch[:1]
        # Clamp count to a sensible max to avoid huge outputs
        try:
            n = int(count)
        except Exception:
            n = 0
        n = max(0, min(n, 161))  # adjust max as you like
        print(ch * n)

    def _env_with_envfile(self) -> dict:
        import os
        env = dict(os.environ)
        env["SYSTEMD_PAGER"] = ""
        env["SYSTEMD_COLORS"] = "0"
        env["ENV_FILE"] = os.path.abspath(os.path.join(os.getcwd(), "env.sh"))
        return env


class ArgParser:

    def __init__(self):
        # Don’t create NodeSetupCLI here (args doesn’t exist yet)
        self.cli = None

    def parser_main(self):
        parser = argparse.ArgumentParser(
            prog='* * * * * * NYM * NODE * CLI * * * * * *',
            description='An interactive tool to download, install, setup and run nym-node',
            epilog='Privacy infrastructure operated by people around the world'
        )
        parser.add_argument("-V","--version", action="version", version='%(prog)s 1.0.0')
        parser.add_argument("-d", "--dev", help='Define github branch, default="develop"',
                            type=str, default='develop')

        subparsers = parser.add_subparsers(help="[-h] shows this help menu")
        parser_install = subparsers.add_parser('install', help='Starts nym-node installation setup CLI',
                                               aliases=['i','I'])
        parser_install.set_defaults(func=self.run_node_installation)

        args = parser.parse_args()

        self.cli = NodeSetupCLI(args)

        try:
            args.func(args)
        except AttributeError as e:
            msg = f"{e}.\nPlease run: ./nym-node-cli --help"
            self.panic(msg)
        except KeyError as e:
            self.panic(f"{e}.")
        except ConnectionError as e:
            msg = f"{e}.\nMake sure you have internet connection or the branch passed to `--dev <BRANCH>` contains this program."
            self.panic(msg)

#    def run_node_installation(self,args):
#        self.cli.run_script(cli.prereqs_install_sh)
#        self.cli.run_script(cli.env_vars_install_sh)
#        self.cli.run_script(cli.node_install_sh)
#        self.cli.run_script(cli.service_config_sh)
#        self.cli._check_gwx_mode() and cli.run_script(cli.nginx_proxy_wss_sh)
#        self.cli.run_nym_node_as_service()
#        self.cli.run_bonding_prompt()
#        self.cli._check_gwx_mode() and cli.run_tunnel_manager_setup()
#        self.cli._check_gwx_mode() and cli.check_wg_enabled() and cli.setup_test_wg_ip_tables()

    def run_node_installation(self, args):
        self.cli.run_script(self.cli.prereqs_install_sh)
        self.cli.run_script(self.cli.env_vars_install_sh)
        self.cli.run_script(self.cli.node_install_sh)
        self.cli.run_script(self.cli.service_config_sh)
        if self.cli._check_gwx_mode():
            self.cli.run_script(self.cli.nginx_proxy_wss_sh)
        self.cli.run_nym_node_as_service()
        self.cli.run_bonding_prompt()
        if self.cli._check_gwx_mode():
            self.cli.run_tunnel_manager_setup()
            if self.cli.check_wg_enabled():
                self.cli.setup_test_wg_ip_tables()


class SystemSafeGuards:
    def __init__(self):
        self.branch = "feature/node-setup-cli"


    def _protect_from_oom(self, score: int = -900):
        try:
            with open("/proc/self/oom_score_adj", "w") as f:
                f.write(str(score))
        except Exception:
            pass

    def _trim_memory(self):
        # Free freeable Python objects and return arenas to the OS if possible
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
        # Limit this Python process to e.g. 2 GiB virtual memory
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
