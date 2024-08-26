#!/venv/bin/python3

import os
import pprint
import requests
from requests.exceptions import HTTPError

DEFAULT_CONFIG_PATH = "~/.nym/nym-api/default/config/config.toml"


# TODO test
def find_bind_addr(config_path=DEFAULT_CONFIG_PATH):
    with open(os.path.expanduser(config_path), "r") as config:
        addr_line = [
            line for line in config.readlines() if line.startswith("bind_address")
        ][0]
        bind_address = addr_line.split()[-1].strip("'")

    return bind_address.split(":")


def check_url(expected_code, url, headers=None):
    if headers is None:
        headers = {}

    # always present
    headers.update({"Accept": "application/json"})

    response = None
    json_data = None

    try:
        response = requests.get(f"http://{url}", headers=headers)
        response.raise_for_status()
    except HTTPError as http_err:
        if response is not None and response.status_code == 404:
            print(f"Error: {http_err}. URL returned 404 Not Found.")
        else:
            print(f"HTTP error occurred: {http_err}")
    except Exception as err:
        print(f"Other error occurred: {err}")
    else:
        response_code = response.status_code

        if "application/json" in response.headers["Content-Type"]:
            try:
                json_data = response.json()

                full_path_info = json_data.get("paths")
                path_list = list(full_path_info.keys())
                if path_list is not None:
                    print(f"{len(path_list)}")
                    print(f"paths found: {path_list}")

                api_spec = None
                # TODO do this for all paths
                example_path = path_list[0]
                if isinstance(full_path_info, dict):
                    api_spec = full_path_info.get(example_path)

                if api_spec is not None:
                    print(f"API spec for `{example_path}`:\n{api_spec}")

            except ValueError:
                print("Response is not valid JSON.")

        else:
            print("Content-Type is not application/json")

        if response_code == expected_code:
            print(f"Expected response code {expected_code} received.")
        else:
            print(
                f"Unexpected response code. Expected {expected_code}, got {response_code}."
            )


def api_status():
    url_list = ["/v1/api-status/build-information"]


if __name__ == "__main__":
    axum_addr, axum_port = find_bind_addr()

    # target_url = f"{axum_addr}:{axum_port}/swagger/"
    swagger_json_url = f"{axum_addr}:{axum_port}/api-docs/openapi.json"

    check_url(200, swagger_json_url)
