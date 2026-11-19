# GitHub access and initial repository setup

Target: **`ducntvt/android-hub`**, private, default branch `main`, integration branch
`develop`. This page describes intended setup; it is not evidence that the remote,
branches, rulesets or workflows have already been created.

## Access

Use a GitHub account that owns the repo or has been invited. The profile URL
`https://github.com/ducntvt` is not itself a repository remote.

With GitHub CLI:

```bash
gh auth login --hostname github.com --web
gh api user --jq .login
gh auth setup-git
gh repo view ducntvt/android-hub --json nameWithOwner,isPrivate,viewerPermission
```

Choose the intended account in the browser. Do not paste a token/password into
chat, docs, scripts or Git remotes. A private-repository 404 can mean either
absence or lack of access. Confirm authentication before assuming it needs creation.
Repository creation under the personal account requires that account's credentials.

Invited users can then clone:

```bash
git clone https://github.com/ducntvt/android-hub.git
cd android-hub
```

## What belongs in Git

Include the three crates, scripts, tests, metrics source, both Cargo.lock files,
maintained docs, architecture records and required build resources (DEX, protocol,
SDK plan and logo). Build resources are not caches. Keep the local app, targets,
credentials, Android data, downloaded SDK/images and QA bundles out of Git.

Historical `docs/reports` may contain local paths or verbose diagnostic logs. Review
that directory before the initial commit. A private repo is still shared with its
collaborators; do not upload credentials, tokens, Google account data or device disks.
Do not discard provenance or break links blindly to reduce the file count.

## Bootstrap (owner only)

First finish/review documentation and inspect the exact initial file list. If the
remote already exists, inspect its contents and history; never force-push over it.
If it does not exist, create an **empty private repo** without auto-generated README
or license so that the first push has a single intended history.

The following illustrates the initial sequence. A coding agent must obtain the
user's explicit commit instruction before executing `git commit`.

```bash
git init -b main
git status --short --ignored
# Stage reviewed files, then inspect the staged list and diff before committing.
git add .gitignore .gitattributes Cargo.toml Cargo.lock README.md README.vi.md \
  CHANGELOG.md CONTRIBUTING.md SECURITY.md THIRD_PARTY_NOTICES.md \
  CLAUDE.md crates scripts tests tools docs
git diff --cached --stat
git diff --cached --check
git commit -m 'chore: initialize Android Hub source'

# Run only after confirming this personal repo does not already exist.
gh repo create ducntvt/android-hub --private --source . --remote origin
git push -u origin main
git switch -c develop
git push -u origin develop
```

No release tag is created during bootstrap. Make the first release separately using
[Releasing](RELEASING.md) after verifying the pushed source.

## Repository settings

- Keep visibility private and default branch `main`.
- Invite collaborators explicitly; use PRs for subsequent changes.
- Prefer checks before merging and block force-push/delete for shared branches.
- Protect published `v*` tags against rewriting/deletion where supported.
- Do not require another reviewer's approval for a solo-maintained repository.
- CI is defined in `.github/workflows/ci.yml`; enable required status checks only
  after the first successful run, selecting the check actually shown on the PR.
- Current binary releases use ad-hoc signatures; no Apple signing secrets are needed.
  Developer ID/notarization is a separate future setup.

Branch/tag ruleset availability depends on repository visibility and the account's
plan; see [GitHub rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets).
