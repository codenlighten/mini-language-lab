# IP records

| Date | Covers | Type | Record hash | Signer | Tag |
|---|---|---|---|---|---|
| 2026-11-26 | Commit `3d03a5c`: a small flow language with a Rust parser, validator and formatter compiled to WebAssembly and a browser editor, source, tests and documentation. No authorship claimed in third-party libraries, tool scaffolding and generated lockfiles. | software, hash-only | [`679d1b657f8578042085fbb176587eb6d3a0412d704dadffc283580005925c2b`](https://iptrust.org/v/679d1b657f8578042085fbb176587eb6d3a0412d704dadffc283580005925c2b) | Gregory J. Ward | `ip-2026-11-26` |

A record is evidence that the committed content existed by the time of its anchor and that the signer made
the statement; it does not create or register any right. Anchoring completes some hours after submission.
Files under `third_party/`, where present, are not part of the record.

## Verifying

Verification needs the evidence package for the record, which the author holds and which is not committed
here. With it in `.iptrust/`:

```
git worktree add /tmp/at-tag-mini-language-lab ip-2026-11-26
cp -r .iptrust /tmp/at-tag-mini-language-lab/
(cd /tmp/at-tag-mini-language-lab && iptrust2 verify 679d1b657f8578042085fbb176587eb6d3a0412d704dadffc283580005925c2b --fetch-headers)
git worktree remove --force /tmp/at-tag-mini-language-lab
```

Expect `verdict: valid` and `content: matched` once both chains have anchored.
