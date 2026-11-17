# Security policy

## Supported versions

tpt-cbor has not been released yet. Until version 1.0.0 is published, only the
`master` branch is supported.

## Reporting a vulnerability

Do not report security problems in a public issue.

Use GitHub's private vulnerability reporting on this repository: open the
**Security** tab and choose **Report a vulnerability**. This keeps the report
visible only to the maintainers until a fix is ready.

Please include:

* the version or commit you tested;
* the input bytes, in hex, that trigger the problem;
* what you observed (a panic, a hang, excess memory use, an incorrect
  acceptance or rejection) and what you expected.

The maintainers aim to acknowledge a report within a week. Fixes are released
with a changelog entry that describes the impact.

## Scope

In scope: panics, out-of-bounds reads, unbounded recursion or allocation, and
any decoder that accepts input that strict mode should reject, or encodes
output that is not canonical.

Out of scope: problems that need a malicious build environment or a compromised
dependency that is not part of this repository.
