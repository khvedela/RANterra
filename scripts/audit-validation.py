"""Audit the isolated codec validator; do not silently waive new findings."""
import json
import os
import pathlib
import subprocess
root = pathlib.Path(__file__).resolve().parent.parent
result = subprocess.run(['go','run','golang.org/x/vuln/cmd/govulncheck@v1.9.0','-json','-test','./...'],
    cwd=root/'validation/free5gc', env={**os.environ,'GOTOOLCHAIN':'go1.27.2'}, text=True, capture_output=True)
output = root/'test-results/validation-go-audit.json'
output.parent.mkdir(exist_ok=True)
output.write_text(result.stdout)
if result.stderr:
    print(result.stderr)
if result.returncode != 0:
    raise SystemExit(f'govulncheck execution failed: {result.returncode}')
decoder = json.JSONDecoder()
remaining = result.stdout.lstrip()
called = set()
configured = False
while remaining:
    record, end = decoder.raw_decode(remaining)
    remaining = remaining[end:].lstrip()
    if 'config' in record:
        configured = record['config'].get('go_version') == 'go1.27.2'
    finding = record.get('finding', {})
    trace = finding.get('trace', [])
    if trace and trace[0].get('function'):
        called.add(finding['osv'])
assert configured, 'Missing audit configuration or unexpected Go toolchain'
# Explicit exception: standalone bounded trusted-frame validator, not application code.
# APER panics cause a controlled test failure; no network listener or imported user data.
allowed = {'GO-2022-1083'}
if called - allowed:
    raise SystemExit(f'New reachable validation advisory: {sorted(called-allowed)}')
if called:
    print('WARNING: isolated validator exception GO-2022-1083 (APER malformed-input panic); see validation/free5gc/README.md.')
else:
    print('No reachable validation advisories; review removal of the APER exception.')
