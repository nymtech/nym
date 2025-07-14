# Outfox

## Notes from Georgio:

### DO NOT INTEGRATE BEFORE:
- Checking the situation with empty Nonces/Additional Data for `ChaCha20Poly1305` Calls.
- Programmatically checking which KEM is used during packet handling, and handling related errors.
- Crypto audit of `src/lion.rs`.
- Updating documentation to replace discrete-log operations with KEM operations.

### Things to optimize:
- Copying and allocations.
- Using libcrux for `ChaCha20`/`ChaCha20Poly1305`
