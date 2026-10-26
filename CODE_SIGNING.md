# Code signing

## Policy

Shape's release binaries are built from this repository by its GitHub Actions
release workflow (`.github/workflows/release.yml`), from a version tag, on
GitHub-hosted runners. Nothing is signed that was not built there.

- **macOS** binaries and `Shape.dmg` (Shape.app) are signed with the
  maintainer's Apple Developer ID certificate and notarized by Apple; the
  DMG has its ticket stapled.
- **Windows** binaries: free code signing provided by
  [SignPath.io](https://about.signpath.io), certificate by
  [SignPath Foundation](https://signpath.org).

**Team.** Committer, reviewer and release approver: Deyan Dobrinov
([@dobrinov](https://github.com/dobrinov)). Every release is a tag the
approver pushes; each signing request is approved by them.

**Privacy.** Shape does not send information to other networked systems
unless the user asks it to, with these exceptions, which can be turned off:

- Once a day it asks GitHub's public API for the latest release, to say when
  a new version is out (`SHAPE_NO_UPDATE_CHECK=1` turns this off).
- The AI features run the AI command-line tool the user installed and signed
  in to (Claude, Codex, Gemini or another the user names); what they send is
  between the user and that service.

The editor runs a server on the user's own machine (`127.0.0.1`) only.
Session logs stay on the user's machine (`SHAPE_NO_LOG=1` turns them off).
There is no telemetry.

## Setting it up (maintainer)

Signing runs in the release workflow once its secrets are set (repository
settings → *Secrets and variables* → *Actions*); until then binaries are
built unsigned.

### macOS: Developer ID and notarization

1. In the Apple Developer account, create a **Developer ID Application**
   certificate (Certificates → +), install it, and export it from Keychain
   Access with its private key as a `.p12` with a password.
2. At <https://account.apple.com> → Sign-In and Security → **App-Specific
   Passwords**, create one for notarization.
3. Run `scripts/apple-signing-secrets.sh path/to/certificate.p12`: it asks
   for the passwords, checks them, and stores these repository secrets:
   - `MACOS_CERT_P12`: the `.p12`, base64
   - `MACOS_CERT_PASSWORD`: the `.p12` password
   - `APPLE_ID`: the Apple ID email of the developer account
   - `APPLE_APP_PASSWORD`: the app-specific password

   The team ID is read from the certificate.

### Windows: SignPath Foundation

1. Apply for the open source program at <https://signpath.org/apply>
   (repository: github.com/dobrinov/shape, license: MIT, this policy).
2. Once accepted, in SignPath: connect the GitHub repository as a trusted
   build system; create the project `shape` with an artifact configuration
   for a **zip file containing `shape.exe`** (a PE file to sign) and the
   signing policy `release-signing`.
3. Add the secret `SIGNPATH_API_TOKEN` (a CI user's API token) and the
   variable `SIGNPATH_ORGANIZATION_ID`; `SIGNPATH_PROJECT_SLUG` and
   `SIGNPATH_POLICY_SLUG` are needed only if they are not `shape` and
   `release-signing`.
