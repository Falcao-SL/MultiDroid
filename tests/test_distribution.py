import hashlib,json,os,re,subprocess,tempfile,tomllib,unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

class Distribution(unittest.TestCase):
 def test_brand_assets_match_official_sources(self):
  subprocess.run(['python3',str(ROOT/'scripts/check-brand.py')],check=True)
 def test_site_is_static_and_has_no_store_badges(self):
  site=(ROOT/'site/index.html').read_text();css=(ROOT/'site/assets/redesign.css').read_text()
  self.assertNotIn('fonts.googleapis.com',css)
  self.assertNotRegex(site,r'<script[^>]+src=')
  self.assertNotIn('iramarfalcao/MultiDroid',site)
  self.assertNotIn('apps.apple.com',site)
  self.assertNotIn('apps.microsoft.com',site)
  self.assertIn('platforms/linux.svg',site)
  self.assertIn('platforms/windows.svg',site)
 def test_all_static_downloads_are_published_assets(self):
  site=(ROOT/'site/index.html').read_text()
  manifest=json.loads((ROOT/'site/releases.json').read_text())
  names={a['url'] for release in manifest.values() for a in release['assets']}
  for url in re.findall(r'href="(https://github.com/Falcao-SL/MultiDroid/releases/download/[^"]+)"',site):
   self.assertIn(url,names)
 def test_version_rejects_unrelated_or_unsafe_tags(self):
  for tag in ('v9.9.9','v0.3.0;echo test','../main'):
   with tempfile.NamedTemporaryFile() as out:
    result=subprocess.run(['python3',str(ROOT/'scripts/release-version.py')],env={**os.environ,'REQUESTED_TAG':tag,'GITHUB_OUTPUT':out.name},capture_output=True)
    self.assertNotEqual(result.returncode,0)
 def test_version_routes_native_and_preview(self):
  native_version=(ROOT/'mac/VERSION').read_text().strip()
  rust_version=tomllib.loads((ROOT/'rust/Cargo.toml').read_text())['workspace']['package']['version'].split('-')[0]
  for tag,native in [('v'+native_version,'true'),('v'+rust_version+'-alpha.4','false')]:
   with tempfile.NamedTemporaryFile() as out:
    subprocess.run(['python3',str(ROOT/'scripts/release-version.py')],env={**os.environ,'REQUESTED_TAG':tag,'GITHUB_OUTPUT':out.name},check=True,capture_output=True)
    self.assertIn('native='+native,Path(out.name).read_text())
 def test_partial_release_is_rejected(self):
  with tempfile.TemporaryDirectory() as directory:
   (Path(directory)/'out').mkdir()
   result=subprocess.run(['python3',str(ROOT/'scripts/verify-release.py')],cwd=directory,env={**os.environ,'RELEASE_TAG':'v0.3.0-alpha.4','NATIVE':'false','SOURCE_COMMIT':'test'},capture_output=True)
   self.assertNotEqual(result.returncode,0)

 def test_production_rejects_partial_platform_selection(self):
  rust=tomllib.loads((ROOT/'rust/Cargo.toml').read_text())['workspace']['package']['version'].split('-')[0]
  with tempfile.NamedTemporaryFile() as out:
   result=subprocess.run(['python3',str(ROOT/'scripts/release-version.py')],env={**os.environ,'REQUESTED_TAG':'v'+rust+'-alpha.4','GITHUB_OUTPUT':out.name,'QA_PLATFORM':'windows','PUBLISH':'true'},capture_output=True)
   self.assertNotEqual(result.returncode,0)
 def test_draft_verification_rejects_incomplete_upload(self):
  with tempfile.TemporaryDirectory() as directory:
   work=Path(directory);(work/'out').mkdir();(work/'out/package.exe').write_bytes(b'example')
   draft=work/'draft.json';draft.write_text(json.dumps({'draft':True,'assets':[]}))
   result=subprocess.run(['python3',str(ROOT/'scripts/verify-uploaded-release.py'),str(draft)],cwd=work,capture_output=True)
   self.assertNotEqual(result.returncode,0)
 def test_draft_verification_accepts_matching_digest(self):
  with tempfile.TemporaryDirectory() as directory:
   work=Path(directory);(work/'out').mkdir();data=b'example';(work/'out/package.exe').write_bytes(data)
   draft=work/'draft.json';draft.write_text(json.dumps({'draft':True,'assets':[{'name':'package.exe','state':'uploaded','size':len(data),'digest':'sha256:'+hashlib.sha256(data).hexdigest()}]}))
   result=subprocess.run(['python3',str(ROOT/'scripts/verify-uploaded-release.py'),str(draft)],cwd=work,capture_output=True)
   self.assertEqual(result.returncode,0,result.stderr.decode())

 def test_site_staging_versions_the_actual_icon_bytes(self):
  with tempfile.TemporaryDirectory() as directory:
   output=Path(directory)/'site'
   subprocess.run(['python3',str(ROOT/'scripts/prepare-site.py'),'--output',str(output)],check=True,capture_output=True)
   digest=hashlib.sha256((ROOT/'site/assets/multidroid-mark.svg').read_bytes()).hexdigest()[:12]
   self.assertIn('assets/multidroid-mark.svg?v='+digest,(output/'index.html').read_text())
   self.assertNotIn('fonts.googleapis.com',(output/'assets/redesign.css').read_text())

if __name__=='__main__':unittest.main()
