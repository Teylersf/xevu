"""Collect the exact locked Rust dependencies' distributed license texts."""
from pathlib import Path
import json,os,re,tomllib,urllib.request,urllib.error
ROOT=Path(__file__).resolve().parents[1]
registry=Path(os.environ.get('CARGO_HOME',Path.home()/'.cargo'))/'registry/src'
packages=tomllib.loads((ROOT/'Cargo.lock').read_text())['package']
parts=['# Third-party notices\n\nThese notices accompany the compiled NULLGRAV preview. Project code is MIT licensed. Embedded fonts retain the OFL notices under assets/fonts/.\n']
count=0
for package in packages:
    if not package.get('source','').startswith('registry+'):continue
    matches=list(registry.glob(f"*/{package['name']}-{package['version']}"))
    if not matches:raise SystemExit(f"Missing fetched package: {package['name']}")
    folder=matches[0];metadata=tomllib.loads((folder/'Cargo.toml').read_text())['package']
    licenses=sorted(p for p in folder.iterdir() if p.is_file() and p.name.upper().startswith(('LICENSE','COPYING','NOTICE')))
    if not licenses and package['name'].startswith('zune-'):
        # These published crates omit their monorepo-root license files.
        # Recover the original texts at the exact crate's recorded source commit.
        vcs=json.loads((folder/'.cargo_vcs_info.json').read_text());commit=vcs['git']['sha1']
        if not re.fullmatch('[0-9a-f]{40}',commit):raise SystemExit('Invalid source commit in crate metadata')
        cache=ROOT/'assets/licenses'/f"{package['name']}-{package['version']}";cache.mkdir(parents=True,exist_ok=True)
        for name in ['LICENSE-MIT','LICENSE-APACHE','LICENSE-ZLIB','COPYRIGHT.md']:
            destination=cache/name
            if not destination.exists():
                try:urllib.request.urlretrieve(f'https://raw.githubusercontent.com/etemesi254/zune-image/{commit}/{name}',destination)
                except urllib.error.HTTPError as error:
                    if error.code!=404:raise
                    continue
            licenses.append(destination)
    if not licenses:raise SystemExit(f"No license texts found for {package['name']}; inspect before packaging")
    parts.append(f"\n## {package['name']} {package['version']}\n\nDeclared license: {metadata.get('license','see included text')}\n")
    for license in licenses:
        parts.append(f"\n### {license.name}\n\n```text\n{license.read_text(encoding='utf-8',errors='replace').rstrip()}\n```\n")
    count+=1
(ROOT/'THIRD_PARTY_NOTICES.md').write_text(''.join(parts),encoding='utf-8',newline='\n')
print(f'Collected distributed license texts for {count} locked third-party packages.')
