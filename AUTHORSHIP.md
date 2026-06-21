# Authorship and provenance

Lightning Git is the sole work of **191-iota**. This file explains how that authorship can be checked independently, so that a copy of the source alone is not enough for someone else to credibly claim it.

## What proves it

**1. The full development history.** This repository contains every commit from all four original packages, with their original timestamps, going back to the first lines of each. The history is the record of the project being built incrementally over time: hundreds of commits, the false starts, the refactors, the test tiers growing. A copier who lifts the final tree has the code but not this record, and cannot fabricate a matching history with consistent dates and content after the fact.

**2. Signed release tags.** Every release is published as an annotated tag signed with 191-iota's SSH key. Only the holder of the corresponding private key can produce a valid signature, and that same key is registered on the [191-iota GitHub account](https://github.com/191-iota), which is what ties the signature to the published identity. A copier cannot re-sign a tag as 191-iota.

**3. Copyright.** The code is released under the MIT License, Copyright (c) 2026 191-iota (see each package's `LICENSE`). MIT permits reuse, but it does not transfer authorship, and the copyright line must be preserved.

## Verifying a signed tag

The public key is committed to this repository in [`.allowed_signers`](.allowed_signers), so verification is self-contained:

```bash
git -c gpg.ssh.allowedSignersFile=.allowed_signers verify-tag v0.1.0
```

A `Good "git" signature for 101976629+191-iota@users.noreply.github.com` line means the tag was signed by 191-iota's key. The same key is on the 191-iota GitHub account, so GitHub also shows a `Verified` badge on signed tags and commits.

To check the development history yourself:

```bash
git log --oneline -- backend     # the full history of one package, before and after consolidation
git log --reverse --format='%ci %an %s' | head   # the earliest commits, with dates and author
```

## For anyone reusing this code

You are welcome to under the MIT terms. Keep the `LICENSE` and its copyright line. This file is not a restriction on reuse; it is a record of origin.
