# Security reporting

This is a private personal project with no formal security support SLA. Use the
latest reviewed source on `main`; older tags are not maintained security branches.

Report a suspected vulnerability privately to the repository owner through an
existing private contact channel. Use GitHub's private vulnerability reporting
only if the owner has enabled it. Do not publish a public issue or disclose
credentials to demonstrate impact.

Include the tag/commit, affected component, reproduction using disposable data,
expected/actual behavior and a redacted error summary. Do not send Google account
passwords, access tokens, Android userdata images or unredacted personal logs.

The app controls local Android processes and disks. Use a separate QA root when
investigating destructive behavior. Do not test against another person's devices,
accounts or data without their permission. See [Setup](docs/SETUP.md) for data paths.
