# Authorship and provenance

HW Watcher is created and copyrighted by **AbnerMSaconi**. Free software cannot technically stop
someone from deleting a notice, so the project combines legal duties with visible marks and
cryptographic proof. Together they make copies easy to recognize and the original easy to prove.

## 1. Legal duties

- The program is GPL-3.0-only (`LICENSE`): every copy or derivative must keep the copyright
  notices, state its changes and ship its full source under the same license.
- `NOTICE` adds terms permitted by GPL v3 section 7: the author attribution shown by the program
  must be preserved, modified versions must be marked as such, and the name may not be used to
  endorse derived products.

## 2. Marks inside every copy

| Where | What |
|---|---|
| `hw-watcher about` | Author, license, original project URL, version, commit, provenance record |
| Control Panel footer | `(c) 2026 AbnerMSaconi · GPL-3.0-only`, and the version in the header |
| Binary | A plain-text record: `strings hw-watcher \| grep hw-watcher-provenance` |
| Source files | `SPDX-License-Identifier` and `Copyright (C)` header on every file |
| `hw-watcher --version` | Version and the exact commit it was built from |

## 3. Proof for official releases

Releases carry up to three independent proofs. Check whichever you can:

```bash
# 1. Integrity
sha256sum -c hw-watcher-0.1.1-linux-x86_64.tar.gz.sha256

# 2. Built by this repository's workflow from a specific commit (Sigstore attestation)
gh attestation verify hw-watcher-0.1.1-linux-x86_64.tar.gz --repo AbnerMSaconi/hw-watcher

# 3. Signed with the project's own release key (works offline)
scripts/verify-release.sh hw-watcher-0.1.1-linux-x86_64.tar.gz
```

The release key is separate from the author's personal commit-signing key and its private half
never leaves the author's machine (it is not a CI secret). The public key and its fingerprint are
in `docs/keys/`. A tarball that fails any of these checks is not an official build.

## 4. Proof of authorship over time

- The public git history and the tagged releases show who wrote what and when.
- Commits and tags signed by the author (`git log --show-signature`) bind that history to the
  author's key. Sign tags with `git tag -s`.
- For legal evidence, registering the software with the national copyright office (in Brazil, the
  INPI "registro de programa de computador") gives an official, dated record of authorship.

## 5. If you find an uncredited copy

Open a "license violation" issue or contact the author with links to the copy. The usual remedy is
simple: restore the notices, publish the source and mark the changes.
