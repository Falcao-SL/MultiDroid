import hashlib,json
from pathlib import Path
root=Path(__file__).resolve().parents[1]
m=json.loads((root/'branding/manifest.json').read_text())
checks={m['source']:m['sourceSHA256'],'docs/design/tokens.json':m['tokensSHA256'],**m['outputs']}
for name,expected in checks.items():
 if hashlib.sha256((root/name).read_bytes()).hexdigest()!=expected:raise SystemExit('Outdated brand asset: '+name+'; run scripts/sync-brand.py')
print('Brand source, tokens, application icons and installer assets are synchronized.')
