# Release validation — VERSION

- Candidate commit:
- Date / tester:
- Windows builds, CPU/GPU, driver, RAM, display/DPI:

## Automated

- [ ] Format and native release build
- [ ] Four core tests
- [ ] Three ignored release network regressions
- [ ] Optional Game Bar build/package (or explicitly not distributed)

## Windows acceptance

- [ ] Complete the matrix in `docs/testing.md`; attach results and exceptions
- [ ] Both modes preserve foreground HWND, keyboard and mouse
- [ ] Three word-art styles, custom PNG and five animation choices
- [ ] Model vision/OCR smoke tests; credentials and failure behavior
- [ ] Multi-monitor/DPI, widget move/resize and auto-hide
- [ ] Game/version/display-mode results added to `docs/compatibility.md`
- [ ] 60-second footprint sample fresh and after repeated use
- [ ] Endurance / resource handles stable over repeated captures
- [ ] No secrets or screenshots in package/logs

## Results

Host hidden private working set / CPU:

Widget visible private working set / CPU:

Optional Game Bar metrics:

Known limitations and unverified scenarios:

Decision: development preview / release candidate / stable (explain).
