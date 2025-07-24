# smolmix poc

At the moment this is very basic but it works with smol files (large ones end up borking at the moment).

```sh
cargo run --example download
```

There is a quick check for httpbin's IPs via `nslookup` and connectivity **not** using the Mixnet:

```sh
cargo run --example check
```
