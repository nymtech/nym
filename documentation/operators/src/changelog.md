# Changelog

This page displays a full list of all the changes during our release cycle from [`v2024.3-eclipse`](https://github.com/nymtech/nym/blob/nym-binaries-v2024.3-eclipse/CHANGELOG.md) onwards. Operators can find here the newest updates together with links to relevant documentation. The list is sorted in a way that the most recent changes are at the top.

## `v2024.3-eclipese`

- Release [Changelog.md](https://github.com/nymtech/nym/blob/nym-binaries-v2024.3-eclipse/CHANGELOG.md)
- The [`nym-node`](nodes/nym-node.md) initial release
- New versioning `1.1.0+nymnode` can be seen as a misnomer, used internally to monitor migration testing, not necessary for operational use. We plan to correct this in a future release to ensure that mixnodes feature correctly in the main API.
- New [VPS specs & configuration](nodes/vps-setup.md) page
- New [configuration page](nodes/configuration.md) with [connectivity setup guide](nodes/configuration.md#connectivity-test-and-configuration), a new requirement for `exit-gateway`
- API endpoints redirection

~~~admonish example collapsible=true title="API endpoint redirection:"
```
http://<IP>:8000/stats        -->  http://<IP>:8000/api/v1/metrics/mixing
http://<IP>:8000/hardware     -->  http://<IP>:8000/api/v1/system-info
http://<IP>:8000/description  -->  http://<IP>:8000/api/v1/description
```
~~~
