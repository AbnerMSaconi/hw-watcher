# Release signing key

Public key of the HW Watcher release signature, in the format read by `ssh-keygen -Y verify`
(`release-signers`).

| | |
|---|---|
| Identity | `release@hw-watcher` |
| Namespace | `hw-watcher-release` |
| Algorithm | ED25519 |
| Fingerprint | `SHA256:QEA7gYjWaZU7mcYl7FWjaJEZaA8gMG43htyc5JZRLd4` |

Verify a release file (needs its `.sig` next to it):

```sh
scripts/verify-release.sh hw-watcher-<version>-linux-x86_64.tar.gz
```

The private key is held only by the author and is never stored in CI. Release 0.1.0 and 0.1.1
were signed after publication; every release also carries a SHA-256 file and a Sigstore build
provenance attestation.
