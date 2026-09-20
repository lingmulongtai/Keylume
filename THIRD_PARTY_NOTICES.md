# Third-party notices

Optional instrument library: **GeneralUser GS 2.0.3**, S. Christian Collins. Downloaded only when requested; not embedded in the installer. Original license and documentation are in `licenses/generaluser/`. See https://www.schristiancollins.com/generaluser.php and `docs/sound-library.md` for the source, integrity check, and supported SoundFont features.

Keylume is an independent application and is not affiliated with Novation or Focusrite. No Novation logo, product photograph, or GPL control-surface implementation is included.

Fonts are bundled for offline use under the SIL Open Font License 1.1:

- IBM Plex Sans JP — IBM Corp. License: [licenses/ibm-plex-sans-jp-OFL.txt](licenses/ibm-plex-sans-jp-OFL.txt).
- Chakra Petch — The Chakra Petch Project Authors. License: [licenses/chakra-petch-OFL.txt](licenses/chakra-petch-OFL.txt).

Full copyright and license texts for the locked Windows Rust dependency graph and production npm packages are in [licenses/THIRD-PARTY-LICENSES.txt](licenses/THIRD-PARTY-LICENSES.txt). Installer and portable ZIP distributions include this file. Regenerate it with `node scripts/collect-licenses.mjs` after installing dependencies.

License texts omitted from crate archives are preserved in `licenses/upstream/`; `sources.json` records their version-specific upstream URLs. The Microsoft WebView2 SDK 1.0.3650.58 loader notice is included separately from the Rust wrapper license. The externally installed WebView2 Runtime is supplied by Microsoft.

The unmodified `selectors` 0.36.1 build dependency is covered by MPL 2.0. Its source is available from [the exact crate archive](https://crates.io/api/v1/crates/selectors/0.36.1/download) and [the upstream revision](https://github.com/servo/stylo/tree/635e1a19d02960588a00e189bd4bd5bdb150ec3d/selectors). Its license is included in the notices above.

Protocol messages were implemented independently from the Novation public programmer documentation. No source code from launchkey-sdk or Ardour was copied.

The embedded FreePats Upright Piano KW small (2019-07-03) SoundFont is published under CC0 1.0. See [source and checksum](resources/piano/SOURCE.md), [original credits](licenses/piano/README.txt), and [CC0 license](licenses/piano/CC0-1.0.txt). The RustySynth renderer is MIT licensed; its complete notice is included in the dependency notices.

The additional FreePats FM Synthesized Piano #2, Old Piano FB small and Upright Piano KW bright small banks are CC0 1.0. Their original README and license texts are in `licenses/piano/`; archive URLs and SHA-256 hashes are in `resources/piano/SOURCE.md`.
