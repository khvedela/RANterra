"""Check the exact packages linked into the standalone Go codec validator."""
import hashlib
import json
import pathlib
import subprocess
root = pathlib.Path('validation/free5gc')
reviewed = {e['name']: e for e in json.loads((root/'LICENSES.json').read_text())}
lines = subprocess.check_output(['go','list','-mod=readonly','-deps','-test','-f',
    '{{if .Module}}{{.Module.Path}} {{.Module.Version}} {{.Module.Dir}}{{end}}','.'],cwd=root,text=True).splitlines()
seen = set()
for line in set(lines):
    if not line or line.startswith('ranterra-validation '):
        continue
    name, version, directory = line.split()
    entry = reviewed[name]
    assert version == entry['version'], f'Unreviewed version: {name}'
    assert entry['license'] in ('Apache-2.0','MIT','BSD-3-Clause','BSD-2-Clause')
    for path in [pathlib.Path(directory)/entry['source_license_file'],root/entry['notice']]:
        assert hashlib.sha256(path.read_bytes()).hexdigest() == entry['sha256'], f'License changed: {name}'
    seen.add(name)
assert seen == set(reviewed)
print(f'Checked {len(seen)} standalone validation dependencies and retained notices.')
