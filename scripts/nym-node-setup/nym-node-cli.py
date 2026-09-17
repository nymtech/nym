#!/usr/bin/python3

__version__ = "1.3.0"
__default_branch__ = "develop"

import os
import re
import sys
import subprocess
import argparse
import tempfile
import shlex
import time
import traceback
from datetime import datetime
from pathlib import Path
from typing import Iterable, Optional, Mapping, Tuple


# ---------------------------------------------------------------------------
# ISO 3166-1 country table, used to validate the node LOCATION before it ever
# reaches nym-node.
#
# nym-node types --location as a real country (celes::Country), not a string,
# so an unparseable value makes the binary exit during --init-only, i.e. AFTER
# the whole install has already run. Worse, the bad value has by then been
# written to env.sh, and a re-run reuses it without prompting. Validating here
# turns that into an immediate, correctable prompt.
#
# Format: alpha2|alpha3|numeric|english short name
# ---------------------------------------------------------------------------
_ISO_3166 = """\
AD|AND|020|Andorra
AE|ARE|784|United Arab Emirates
AF|AFG|004|Afghanistan
AG|ATG|028|Antigua and Barbuda
AI|AIA|660|Anguilla
AL|ALB|008|Albania
AM|ARM|051|Armenia
AO|AGO|024|Angola
AQ|ATA|010|Antarctica
AR|ARG|032|Argentina
AS|ASM|016|American Samoa
AT|AUT|040|Austria
AU|AUS|036|Australia
AW|ABW|533|Aruba
AX|ALA|248|Åland Islands
AZ|AZE|031|Azerbaijan
BA|BIH|070|Bosnia and Herzegovina
BB|BRB|052|Barbados
BD|BGD|050|Bangladesh
BE|BEL|056|Belgium
BF|BFA|854|Burkina Faso
BG|BGR|100|Bulgaria
BH|BHR|048|Bahrain
BI|BDI|108|Burundi
BJ|BEN|204|Benin
BL|BLM|652|Saint Barthélemy
BM|BMU|060|Bermuda
BN|BRN|096|Brunei Darussalam
BO|BOL|068|Bolivia, Plurinational State of
BQ|BES|535|Bonaire, Sint Eustatius and Saba
BR|BRA|076|Brazil
BS|BHS|044|Bahamas
BT|BTN|064|Bhutan
BV|BVT|074|Bouvet Island
BW|BWA|072|Botswana
BY|BLR|112|Belarus
BZ|BLZ|084|Belize
CA|CAN|124|Canada
CC|CCK|166|Cocos (Keeling) Islands
CD|COD|180|Congo, The Democratic Republic of the
CF|CAF|140|Central African Republic
CG|COG|178|Congo
CH|CHE|756|Switzerland
CI|CIV|384|Côte d'Ivoire
CK|COK|184|Cook Islands
CL|CHL|152|Chile
CM|CMR|120|Cameroon
CN|CHN|156|China
CO|COL|170|Colombia
CR|CRI|188|Costa Rica
CU|CUB|192|Cuba
CV|CPV|132|Cabo Verde
CW|CUW|531|Curaçao
CX|CXR|162|Christmas Island
CY|CYP|196|Cyprus
CZ|CZE|203|Czechia
DE|DEU|276|Germany
DJ|DJI|262|Djibouti
DK|DNK|208|Denmark
DM|DMA|212|Dominica
DO|DOM|214|Dominican Republic
DZ|DZA|012|Algeria
EC|ECU|218|Ecuador
EE|EST|233|Estonia
EG|EGY|818|Egypt
EH|ESH|732|Western Sahara
ER|ERI|232|Eritrea
ES|ESP|724|Spain
ET|ETH|231|Ethiopia
FI|FIN|246|Finland
FJ|FJI|242|Fiji
FK|FLK|238|Falkland Islands (Malvinas)
FM|FSM|583|Micronesia, Federated States of
FO|FRO|234|Faroe Islands
FR|FRA|250|France
GA|GAB|266|Gabon
GB|GBR|826|United Kingdom
GD|GRD|308|Grenada
GE|GEO|268|Georgia
GF|GUF|254|French Guiana
GG|GGY|831|Guernsey
GH|GHA|288|Ghana
GI|GIB|292|Gibraltar
GL|GRL|304|Greenland
GM|GMB|270|Gambia
GN|GIN|324|Guinea
GP|GLP|312|Guadeloupe
GQ|GNQ|226|Equatorial Guinea
GR|GRC|300|Greece
GS|SGS|239|South Georgia and the South Sandwich Islands
GT|GTM|320|Guatemala
GU|GUM|316|Guam
GW|GNB|624|Guinea-Bissau
GY|GUY|328|Guyana
HK|HKG|344|Hong Kong
HM|HMD|334|Heard Island and McDonald Islands
HN|HND|340|Honduras
HR|HRV|191|Croatia
HT|HTI|332|Haiti
HU|HUN|348|Hungary
ID|IDN|360|Indonesia
IE|IRL|372|Ireland
IL|ISR|376|Israel
IM|IMN|833|Isle of Man
IN|IND|356|India
IO|IOT|086|British Indian Ocean Territory
IQ|IRQ|368|Iraq
IR|IRN|364|Iran, Islamic Republic of
IS|ISL|352|Iceland
IT|ITA|380|Italy
JE|JEY|832|Jersey
JM|JAM|388|Jamaica
JO|JOR|400|Jordan
JP|JPN|392|Japan
KE|KEN|404|Kenya
KG|KGZ|417|Kyrgyzstan
KH|KHM|116|Cambodia
KI|KIR|296|Kiribati
KM|COM|174|Comoros
KN|KNA|659|Saint Kitts and Nevis
KP|PRK|408|Korea, Democratic People's Republic of
KR|KOR|410|Korea, Republic of
KW|KWT|414|Kuwait
KY|CYM|136|Cayman Islands
KZ|KAZ|398|Kazakhstan
LA|LAO|418|Lao People's Democratic Republic
LB|LBN|422|Lebanon
LC|LCA|662|Saint Lucia
LI|LIE|438|Liechtenstein
LK|LKA|144|Sri Lanka
LR|LBR|430|Liberia
LS|LSO|426|Lesotho
LT|LTU|440|Lithuania
LU|LUX|442|Luxembourg
LV|LVA|428|Latvia
LY|LBY|434|Libya
MA|MAR|504|Morocco
MC|MCO|492|Monaco
MD|MDA|498|Moldova, Republic of
ME|MNE|499|Montenegro
MF|MAF|663|Saint Martin (French part)
MG|MDG|450|Madagascar
MH|MHL|584|Marshall Islands
MK|MKD|807|North Macedonia
ML|MLI|466|Mali
MM|MMR|104|Myanmar
MN|MNG|496|Mongolia
MO|MAC|446|Macao
MP|MNP|580|Northern Mariana Islands
MQ|MTQ|474|Martinique
MR|MRT|478|Mauritania
MS|MSR|500|Montserrat
MT|MLT|470|Malta
MU|MUS|480|Mauritius
MV|MDV|462|Maldives
MW|MWI|454|Malawi
MX|MEX|484|Mexico
MY|MYS|458|Malaysia
MZ|MOZ|508|Mozambique
NA|NAM|516|Namibia
NC|NCL|540|New Caledonia
NE|NER|562|Niger
NF|NFK|574|Norfolk Island
NG|NGA|566|Nigeria
NI|NIC|558|Nicaragua
NL|NLD|528|Netherlands
NO|NOR|578|Norway
NP|NPL|524|Nepal
NR|NRU|520|Nauru
NU|NIU|570|Niue
NZ|NZL|554|New Zealand
OM|OMN|512|Oman
PA|PAN|591|Panama
PE|PER|604|Peru
PF|PYF|258|French Polynesia
PG|PNG|598|Papua New Guinea
PH|PHL|608|Philippines
PK|PAK|586|Pakistan
PL|POL|616|Poland
PM|SPM|666|Saint Pierre and Miquelon
PN|PCN|612|Pitcairn
PR|PRI|630|Puerto Rico
PS|PSE|275|Palestine, State of
PT|PRT|620|Portugal
PW|PLW|585|Palau
PY|PRY|600|Paraguay
QA|QAT|634|Qatar
RE|REU|638|Réunion
RO|ROU|642|Romania
RS|SRB|688|Serbia
RU|RUS|643|Russian Federation
RW|RWA|646|Rwanda
SA|SAU|682|Saudi Arabia
SB|SLB|090|Solomon Islands
SC|SYC|690|Seychelles
SD|SDN|729|Sudan
SE|SWE|752|Sweden
SG|SGP|702|Singapore
SH|SHN|654|Saint Helena, Ascension and Tristan da Cunha
SI|SVN|705|Slovenia
SJ|SJM|744|Svalbard and Jan Mayen
SK|SVK|703|Slovakia
SL|SLE|694|Sierra Leone
SM|SMR|674|San Marino
SN|SEN|686|Senegal
SO|SOM|706|Somalia
SR|SUR|740|Suriname
SS|SSD|728|South Sudan
ST|STP|678|Sao Tome and Principe
SV|SLV|222|El Salvador
SX|SXM|534|Sint Maarten (Dutch part)
SY|SYR|760|Syrian Arab Republic
SZ|SWZ|748|Eswatini
TC|TCA|796|Turks and Caicos Islands
TD|TCD|148|Chad
TF|ATF|260|French Southern Territories
TG|TGO|768|Togo
TH|THA|764|Thailand
TJ|TJK|762|Tajikistan
TK|TKL|772|Tokelau
TL|TLS|626|Timor-Leste
TM|TKM|795|Turkmenistan
TN|TUN|788|Tunisia
TO|TON|776|Tonga
TR|TUR|792|Türkiye
TT|TTO|780|Trinidad and Tobago
TV|TUV|798|Tuvalu
TW|TWN|158|Taiwan, Province of China
TZ|TZA|834|Tanzania, United Republic of
UA|UKR|804|Ukraine
UG|UGA|800|Uganda
UM|UMI|581|United States Minor Outlying Islands
US|USA|840|United States
UY|URY|858|Uruguay
UZ|UZB|860|Uzbekistan
VA|VAT|336|Holy See (Vatican City State)
VC|VCT|670|Saint Vincent and the Grenadines
VE|VEN|862|Venezuela, Bolivarian Republic of
VG|VGB|092|Virgin Islands, British
VI|VIR|850|Virgin Islands, U.S.
VN|VNM|704|Viet Nam
VU|VUT|548|Vanuatu
WF|WLF|876|Wallis and Futuna
WS|WSM|882|Samoa
YE|YEM|887|Yemen
YT|MYT|175|Mayotte
ZA|ZAF|710|South Africa
ZM|ZMB|894|Zambia
ZW|ZWE|716|Zimbabwe
"""

# Names operators actually type that ISO 3166 does not carry.
_LOCATION_ALIASES = {
    "uk": "GB", "england": "GB", "scotland": "GB", "wales": "GB",
    "northern ireland": "GB", "great britain": "GB", "britain": "GB",
    "usa": "US", "u.s.": "US", "u.s.a.": "US", "america": "US",
    "united states of america": "US",
    "czech republic": "CZ", "czechrepublic": "CZ",
    "south korea": "KR", "republic of korea": "KR", "north korea": "KP",
    "russia": "RU", "holland": "NL", "uae": "AE", "emirates": "AE",
    "vietnam": "VN", "laos": "LA", "syria": "SY", "iran": "IR",
    "bolivia": "BO", "tanzania": "TZ", "venezuela": "VE", "moldova": "MD",
    "macedonia": "MK", "north macedonia": "MK", "brunei": "BN",
    "cape verde": "CV", "ivory coast": "CI", "cote d ivoire": "CI",
    "swaziland": "SZ", "burma": "MM", "east timor": "TL", "vatican": "VA",
    "palestine": "PS", "turkey": "TR", "turkiye": "TR",
    "hong kong": "HK", "macau": "MO", "taiwan": "TW",
}


def _build_location_index():
    """alpha2 -> name, plus every accepted spelling -> alpha2."""
    names = {}
    lookup = {}
    for line in _ISO_3166.strip().splitlines():
        a2, a3, num, name = line.split("|")
        names[a2] = name
        for key in (a2, a3, num, name):
            lookup[key.lower()] = a2
        # tolerate "Korea, Republic of" typed as "korea republic of"
        lookup[name.lower().replace(",", "").replace(".", "")] = a2
    for alias, a2 in _LOCATION_ALIASES.items():
        lookup[alias] = a2
    return names, lookup


_LOCATION_NAMES, _LOCATION_LOOKUP = _build_location_index()


class NodeSetupCLI:
    """All CLI main functions"""

    def __init__(self, args):
        self.branch = args.dev
        self.welcome_message = self.print_welcome_message()
        self.mode = self._get_or_prompt_mode(args)

        # Resolve WireGuard up front (CLI > env > env.sh > prompt) so we can
        # derive the full capability set before fetching anything.
        self.wg_enabled = self.check_wg_enabled(args)

        # --- Derive capabilities from mode + WireGuard ---
        #
        # Ground truth of how Nym node roles compose:
        #   * every exit-gateway also serves as an entry-gateway
        #   * every WireGuard node routes exit traffic, so it is effectively an
        #     exit-gateway (and therefore also an entry-gateway)
        #   * a non-WireGuard entry-gateway serves entry traffic only
        #
        # From that:
        #   needs_quic       -> any gateway (entry or exit) runs a QUIC bridge
        #   needs_exit_setup -> exit-gateway OR any WireGuard node
        #                       (nginx/WSS, NTM routing, exit-policy iptables)
        #   needs_ufw        -> only where NTM does NOT manage the firewall:
        #                       mixnodes, and entry-only nodes without WireGuard
        is_mix = self.mode == "mixnode"
        is_entry = self.mode == "entry-gateway"
        is_exit = self.mode == "exit-gateway"

        self.needs_quic = is_entry or is_exit
        self.needs_exit_setup = is_exit or self.wg_enabled
        self.needs_ufw = is_mix or (is_entry and not self.wg_enabled)

        # Inform the operator when an entry-gateway is promoted to an effective
        # exit-gateway purely because WireGuard was enabled.
        if is_entry and self.wg_enabled:
            print(
                "\n[INFO] WireGuard is enabled on an entry-gateway.\n"
                "       WireGuard nodes route exit traffic, so this node will be\n"
                "       set up as a full exit-gateway (nginx/WSS, routing, exit\n"
                "       policy and QUIC) and listed as both entry and exit in the app.\n"
            )

        # --- Base scripts, always needed ---
        self.prereqs_install_sh = self.fetch_script("nym-node-prereqs-install.sh")
        self.node_install_sh = self.fetch_script("nym-node-install.sh")
        self.service_config_sh = self.fetch_script("setup-systemd-service-file.sh")
        self.start_node_systemd_service_sh = self.fetch_script("start-node-systemd-service.sh")

        # --- Conditional scripts ---
        self.landing_page_html = None
        self.nginx_proxy_wss_sh = None
        self.tunnel_manager_sh = None
        self.quic_bridge_deployment_sh = None

        if self.needs_exit_setup:
            self.landing_page_html = self.fetch_script("landing-page.html")
            self.nginx_proxy_wss_sh = self.fetch_script("setup-nginx-proxy-wss.sh")
            self.tunnel_manager_sh = self.fetch_script("network_tunnel_manager.sh")

        if self.needs_quic:
            self.quic_bridge_deployment_sh = self.fetch_script("quic_bridge_deployment.sh")


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
    
    def _coerce_ssh_port(self, value) -> str:
        sval = str(value).strip() if value is not None else ""
        if not sval:
            sval = "22"
        if not sval.isdigit():
            raise ValueError(f"Invalid SSH port: {sval!r}. Expected integer 1..65535.")
        port = int(sval, 10)
        if not 1 <= port <= 65535:
            raise ValueError(f"Invalid SSH port: {port}. Expected integer 1..65535.")
        return str(port)

    def _coerce_location(self, value) -> str:
        """Validate LOCATION against ISO 3166-1 and normalise to alpha-2.

        nym-node parses --location into a real country type, so anything it
        cannot resolve aborts the node during init. Catch it here, while the
        operator can still fix it.
        """
        raw = str(value).strip() if value is not None else ""
        if not raw:
            raise ValueError(
                "Location is required. Give an ISO country code or name, "
                "e.g. 'CH', 'CHE' or 'Switzerland'."
            )

        key = " ".join(raw.lower().replace(",", " ").replace(".", " ").split())
        a2 = _LOCATION_LOOKUP.get(key)
        if a2:
            return a2

        # Not a country. Offer the closest matches rather than just refusing.
        import difflib
        candidates = difflib.get_close_matches(
            key, [n.lower() for n in _LOCATION_NAMES.values()], n=2, cutoff=0.75
        )
        hint = ""
        if candidates:
            suggestions = []
            for cand in candidates:
                for code, name in _LOCATION_NAMES.items():
                    if name.lower() == cand:
                        suggestions.append(f"{code} ({name})")
                        break
            hint = " Did you mean: " + ", ".join(suggestions) + "?"

        raise ValueError(
            f"'{raw}' is not an ISO 3166 country.{hint}\n"
            "  LOCATION is the node's physical COUNTRY, not a city or region.\n"
            "  Accepted: alpha-2 'CH', alpha-3 'CHE', numeric '756', "
            "or the country name 'Switzerland'."
        )

    def _resolve_field(self, args, existing, arg_name, env_key, prompt, *, default=None, validator=None):
        cli_val = getattr(args, arg_name, None)

        if cli_val is not None:
            source = "cli"
            value = str(cli_val).strip()
        elif existing.get(env_key):
            source = "env"
            value = str(existing[env_key]).strip()
        else:
            source = "prompt"
            entered = input(prompt).strip()
            value = entered if entered else (default if default is not None else "")

        if not validator:
            return value

        # Re-prompt on a bad value instead of persisting it. A value that came
        # from env.sh or a CLI flag used to be reused verbatim on every re-run,
        # so a typo could only be fixed by hand-editing env.sh.
        while True:
            try:
                return validator(value)
            except ValueError as err:
                print(f"\n[ERROR] {err}\n")
                if not sys.stdin.isatty():
                    raise SystemExit(1)
                if source == "env":
                    print(f"  (the invalid value came from env.sh: {env_key}=\"{value}\")\n")
                value = input(prompt).strip()
                if not value and default is not None:
                    value = default
                source = "prompt"

    def ensure_env_values(self, args):
        """Collect env vars from args or prompt interactively, then save to env.sh."""
        env_file = Path("env.sh")
        fields = [
            ("hostname", "HOSTNAME", "Enter hostname (if you don't use a DNS, press enter): ", None, None),
            ("location", "LOCATION", "Enter node location - ISO country code or name (e.g. CH, CHE or Switzerland): ", None, self._coerce_location),
            ("email", "EMAIL", "Enter your email: ", None, None),
            ("moniker", "MONIKER", "Enter node public moniker (visible in explorer & NymVPN app): ", None, None),
            ("description", "DESCRIPTION", "Enter short node public description: ", None, None),
            ("host_ssh_port", "HOST_SSH_PORT", "Enter host SSH port (press enter for default port 22): ", "22", self._coerce_ssh_port),
        ]

        existing = self._read_env_file(env_file)
        updated = {}

        for arg_name, key, prompt, default, validator in fields:
            value = self._resolve_field(
                args,
                existing,
                arg_name,
                key,
                prompt,
                default=default,
                validator=validator,
            )
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
        """Fetch a required script over HTTPS.

        Uses Python's urllib rather than shelling out to wget/curl: on a fresh
        machine those tools are installed *by* the prereqs script, which runs
        after this constructor, so depending on them here caused intermittent
        "script not downloaded" failures (notably NTM). urllib is always present
        with the interpreter, and we retry to ride out transient network blips.
        """
        import urllib.request
        import urllib.error

        # print header only the first time
        if not getattr(self, "_fetched_once", False):
            print("\n* * * Fetching required scripts * * *")
            self._fetched_once = True

        url = self._return_script_url(script_name)
        print(f"Fetching file from: {url}")

        last_err = None
        for attempt in range(1, 4):
            try:
                req = urllib.request.Request(url, headers={"User-Agent": "nym-node-cli"})
                with urllib.request.urlopen(req, timeout=30) as resp:
                    data = resp.read().decode("utf-8")
                if not data.strip():
                    raise RuntimeError("empty response body")
                print(f"Downloaded {len(data)} bytes.")
                return data
            except (urllib.error.URLError, urllib.error.HTTPError, RuntimeError, TimeoutError) as e:
                last_err = e
                print(f"[WARN] fetch attempt {attempt}/3 failed for {script_name}: {e}")
                time.sleep(2 * attempt)

        raise RuntimeError(f"Failed to fetch {url} after 3 attempts: {last_err}")

    def _return_script_url(self, script_init_name):
        """Dictionary pointing to scripts url returning value according to a passed key"""
        github_raw_nymtech_nym_scripts_url = f"https://raw.githubusercontent.com/nymtech/nym/refs/heads/{self.branch}/scripts/"
        scripts_urls = {
                "nym-node-prereqs-install.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/nym-node-prereqs-install.sh",
                "nym-node-install.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/nym-node-install.sh",
                "setup-systemd-service-file.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/setup-systemd-service-file.sh",
                "start-node-systemd-service.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/start-node-systemd-service.sh",
                "setup-nginx-proxy-wss.sh": f"{github_raw_nymtech_nym_scripts_url}nym-node-setup/setup-nginx-proxy-wss.sh",
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

    def check_wg_enabled(self, args=None):
        """Determine if WireGuard is enabled; precedence: CLI > env > env.sh > prompt. Persist normalized value."""

        env_file = os.path.join(os.getcwd(), "env.sh")

        def norm(v):
            return "true" if str(v).strip().lower() == "true" else "false"

        # WireGuard is a gateway concept only; mixnodes never route WG traffic.
        if getattr(self, "mode", None) == "mixnode":
            os.environ["WIREGUARD"] = "false"
            return False

        val = None

        # CLI argument
        if args and getattr(args, "wireguard_enabled", None) is not None:
            val = norm(getattr(args, "wireguard_enabled"))
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


    def setup_ufw(self):
        """Configure ufw for nodes NOT managed by the network tunnel manager.

        Applies only to mixnodes and entry-only (non-WireGuard) gateways. Exit
        gateways and WireGuard nodes are excluded because NTM owns their firewall
        via complete_networking_configuration; layering ufw on top would clash.
        """
        print("\n* * * Setting up firewall using ufw * * *")

        ssh_port = os.environ.get("HOST_SSH_PORT", "22")

        # Base rules common to every ufw-managed node.
        rules = [
            f"{ssh_port}/tcp",   # SSH (operator-controlled)
            "80/tcp",            # HTTP
            "443/tcp",           # HTTPS
            "1789/tcp",          # Nym mixnet
            "1790/tcp",          # Nym mixnet
            "8080/tcp",          # nym-node HTTP API
            "9000/tcp",          # clients port
        ]

        # Entry gateways (non-WireGuard) additionally expose the WSS port.
        if self.mode == "entry-gateway":
            rules.append("9001/tcp")  # WSS

        script_lines = [
            "#!/usr/bin/env bash",
            "set -euo pipefail",
            "export DEBIAN_FRONTEND=noninteractive",
            "echo 'y' | ufw enable || ufw --force enable",
        ]
        for rule in rules:
            script_lines.append(f"ufw allow {rule}")
        script_lines.append("ufw reload")
        script_lines.append("ufw status verbose")

        self.run_script("\n".join(script_lines) + "\n")

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
        # NETWORK_DEVICE remains the backward-compatible override for both families
        uplink_updates = {}
        if getattr(args, "uplink_dev", None):
            uplink_updates["NETWORK_DEVICE"] = args.uplink_dev
        if getattr(args, "uplink_dev_v4", None):
            uplink_updates["NETWORK_DEVICE_V4"] = args.uplink_dev_v4
        if getattr(args, "uplink_dev_v6", None):
            uplink_updates["NETWORK_DEVICE_V6"] = args.uplink_dev_v6
        if uplink_updates:
            os.environ.update(uplink_updates)
            self._upsert_env_vars(uplink_updates)
        self.run_script(self.prereqs_install_sh)
        self.run_script(self.node_install_sh)
        self.run_script(self.service_config_sh)

        # nginx reverse proxy + WSS: only nodes doing exit setup (exit-gateway
        # or any WireGuard node) serve the landing page and WSS endpoint.
        if self.needs_exit_setup:
            self.run_script(self.nginx_proxy_wss_sh)

        # Firewall: ufw is only used where NTM does NOT manage iptables, i.e.
        # mixnodes and entry-only nodes without WireGuard. On exit / WireGuard
        # nodes, NTM's complete_networking_configuration owns the firewall and
        # running ufw on top would conflict with its rules.
        if self.needs_ufw:
            self.setup_ufw()

        self.run_nym_node_as_service()
        self.run_bonding_prompt()

        # Exit / WireGuard nodes: NTM routing, then (for WireGuard) exit-policy
        # iptables. QUIC runs on any gateway.
        if self.needs_exit_setup:
            self.run_tunnel_manager_setup()
            if self.wg_enabled:
                self.setup_test_wg_ip_tables()

        if self.needs_quic:
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
        install_parser.add_argument(
            "--location",
            help="Node physical country: ISO 3166 alpha-2 (CH), alpha-3 (CHE), numeric (756) or name (Switzerland)",
        )
        install_parser.add_argument("--email", help="Contact email for the node operator")
        install_parser.add_argument("--moniker", help="Public moniker displayed in explorer & NymVPN app")
        install_parser.add_argument("--description", help="Short public description of the node")
        install_parser.add_argument("--public-ip", help="External IPv4 address (autodetected if omitted)")

        install_parser.add_argument(
            "--host-ssh-port",
            type=int,
            help="Host SSH port to allow in the firewall (default: 22)",
        )

        install_parser.add_argument("--nym-node-binary", help="URL for nym-node binary (autodetected if omitted)")
        install_parser.add_argument(
            "--uplink-dev",
            help="Backward-compatible override for both IPv4 and IPv6 uplinks, e.g. 'eth0'",
        )

        install_parser.add_argument(
            "--uplink-dev-v4",
            help="Override IPv4 uplink interface used for NAT/FORWARD, e.g. 'eth0'",
        )

        install_parser.add_argument(
            "--uplink-dev-v6",
            help="Override IPv6 uplink interface used for NAT/FORWARD, e.g. 'eth1'",
        )
        
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