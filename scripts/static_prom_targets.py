import json
import os


ips = [
    "2.221.182.179",
    "54.232.20.104",
    "15.237.112.155",
    "54.93.108.209",
    "13.38.74.100",
    "15.237.93.154",
    "18.156.175.57",
    "3.76.123.170",
]

port_range = range(18000, 18999)


def make_prom_target(ip, port, env):
    return {
        "targets": [f"{ip}:{port}"],
        "labels": {
            "mixnet_env": env,
        },
    }


if __name__ == "__main__":
    outfile = "/tmp/tmp_static_prom_tragets.json"
    outlink = "/tmp/static_prom_tragets.json"
    targets = []
    for ip in ips:
        for port in port_range:
            targets.append(make_prom_target(ip, port, "performance"))

    with open(outfile, "w") as f:
        json.dump(targets, f)

    os.chmod(outfile, 0o777)
    os.rename(outfile, outlink)
    os.chmod(outlink, 0o777)

    print(f"Prometheus -> {len(targets)} targets written to {outlink}")
